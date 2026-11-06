use axum::{Router, extract::DefaultBodyLimit, routing::get};
use tower_http::trace::TraceLayer;

use crate::{auth_handlers, handlers, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/ready", get(handlers::readiness_check))
        .route(
            "/api/auth/register",
            axum::routing::post(auth_handlers::register),
        )
        .route("/api/auth/login", axum::routing::post(auth_handlers::login))
        .route("/api/auth/me", get(auth_handlers::current_user))
        .route("/api/auth/csrf", get(auth_handlers::csrf_token))
        .route(
            "/api/auth/logout",
            axum::routing::post(auth_handlers::logout),
        )
        .route(
            "/api/tasks",
            get(handlers::list_tasks).post(handlers::create_task),
        )
        .route(
            "/api/tasks/{id}",
            get(handlers::get_task)
                .put(handlers::update_task)
                .delete(handlers::delete_task),
        )
        .fallback(handlers::route_not_found)
        .method_not_allowed_fallback(handlers::method_not_allowed)
        .with_state(state)
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod postgres_auth_integration_tests {
    use std::{net::SocketAddr, str::FromStr};

    use axum::{
        Router,
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{Method, Request, Response, StatusCode, header},
    };
    use serde_json::{Value, json};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::build_router;
    use crate::{
        auth::{hash_password, hash_token, new_token},
        state::{AppState, AuthConfig},
    };

    const PASSWORD: &str = "StrongPassword123!";

    #[tokio::test]
    #[ignore = "requires a disposable loopback PostgreSQL database via NEXORA_AUTH_TEST_DATABASE_URL"]
    async fn authentication_csrf_and_task_ownership_flow() {
        let database_url = std::env::var("NEXORA_AUTH_TEST_DATABASE_URL")
            .expect("set NEXORA_AUTH_TEST_DATABASE_URL to a disposable local PostgreSQL database");
        let options =
            PgConnectOptions::from_str(&database_url).expect("parse local test database URL");
        assert!(
            matches!(options.get_host(), "localhost" | "127.0.0.1" | "::1"),
            "authentication integration test is restricted to loopback PostgreSQL"
        );
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .expect("connect to the explicitly configured local test database");
        let migrations = sqlx::migrate!("./migrations");
        migrations.run(&pool).await.expect(
            "apply migrations; existing unowned tasks cause the ownership migration to stop safely",
        );
        migrations
            .run(&pool)
            .await
            .expect("repeated startup must not reapply completed migrations");

        let suffix = Uuid::new_v4();
        let email_a = format!("user-a-{suffix}@example.test");
        let email_b = format!("user-b-{suffix}@example.test");
        let app = build_router(AppState::new(
            pool.clone(),
            AuthConfig {
                cookie_name: "nexora_session".to_string(),
                cookie_secure: false,
                session_ttl_hours: 1,
                trust_proxy_headers: false,
            },
            hash_password("invalid-login-timing-placeholder".to_string())
                .await
                .unwrap(),
        ));

        let health = send(&app, Method::GET, "/health", None, None, None).await;
        assert_eq!(health.status(), StatusCode::OK);
        let readiness = send(&app, Method::GET, "/ready", None, None, None).await;
        assert_eq!(readiness.status(), StatusCode::OK);

        let malformed_json = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 45678))))
                    .body(Body::from("{"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(malformed_json.status(), StatusCode::BAD_REQUEST);

        let invalid_email = send(
            &app,
            Method::POST,
            "/api/auth/register",
            None,
            None,
            Some(json!({"email":"invalid", "password":PASSWORD})),
        )
        .await;
        assert_eq!(invalid_email.status(), StatusCode::BAD_REQUEST);
        let weak_password = send(
            &app,
            Method::POST,
            "/api/auth/register",
            None,
            None,
            Some(json!({"email":email_a, "password":"weak"})),
        )
        .await;
        assert_eq!(weak_password.status(), StatusCode::BAD_REQUEST);

        let register_a = send(
            &app,
            Method::POST,
            "/api/auth/register",
            None,
            None,
            Some(json!({"email":email_a, "password":PASSWORD})),
        )
        .await;
        assert_eq!(register_a.status(), StatusCode::CREATED);
        let cookie_a = session_cookie(&register_a);
        let body_a = response_json(register_a).await;
        assert_eq!(body_a["user"]["email"], email_a);
        assert!(body_a["user"].get("password_hash").is_none());
        let owner_id: Uuid = body_a["user"]["id"]
            .as_str()
            .expect("user id")
            .parse()
            .expect("valid user UUID");
        let csrf_a = csrf_token(&app, &cookie_a).await;

        let overlong_title =
            sqlx::query("INSERT INTO tasks (id, user_id, title) VALUES ($1, $2, $3)")
                .bind(Uuid::new_v4())
                .bind(owner_id)
                .bind("x".repeat(201))
                .execute(&pool)
                .await;
        assert!(
            overlong_title.is_err(),
            "database must enforce task title length"
        );
        let missing_owner =
            sqlx::query("INSERT INTO tasks (id, user_id, title) VALUES ($1, $2, $3)")
                .bind(Uuid::new_v4())
                .bind(Uuid::new_v4())
                .bind("No owner");
        assert!(
            missing_owner.execute(&pool).await.is_err(),
            "database must enforce task ownership foreign key"
        );

        let duplicate = send(
            &app,
            Method::POST,
            "/api/auth/register",
            None,
            None,
            Some(json!({"email":email_a.to_uppercase(), "password":PASSWORD})),
        )
        .await;
        assert_eq!(duplicate.status(), StatusCode::CONFLICT);

        let unauthenticated = send(&app, Method::GET, "/api/tasks", None, None, None).await;
        assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
        let missing_csrf = send(
            &app,
            Method::POST,
            "/api/tasks",
            Some(&cookie_a),
            None,
            Some(json!({"title":"Must be protected"})),
        )
        .await;
        assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);
        let invalid_csrf = send(
            &app,
            Method::POST,
            "/api/tasks",
            Some(&cookie_a),
            Some("wrong-token"),
            Some(json!({"title":"Must be protected"})),
        )
        .await;
        assert_eq!(invalid_csrf.status(), StatusCode::FORBIDDEN);

        let invalid_uuid = send(
            &app,
            Method::GET,
            "/api/tasks/not-a-uuid",
            Some(&cookie_a),
            None,
            None,
        )
        .await;
        assert_eq!(invalid_uuid.status(), StatusCode::BAD_REQUEST);

        let invalid_task = send(
            &app,
            Method::POST,
            "/api/tasks",
            Some(&cookie_a),
            Some(&csrf_a),
            Some(json!({"title":"   "})),
        )
        .await;
        assert_eq!(invalid_task.status(), StatusCode::BAD_REQUEST);

        let create_a = send(
            &app,
            Method::POST,
            "/api/tasks",
            Some(&cookie_a),
            Some(&csrf_a),
            Some(json!({"title":"User A task", "user_id":Uuid::new_v4()})),
        )
        .await;
        assert_eq!(create_a.status(), StatusCode::CREATED);
        let task_a = response_json(create_a).await;
        let task_id = task_a["id"].as_str().expect("task id").to_string();
        let own_list = send(
            &app,
            Method::GET,
            "/api/tasks?page=1&limit=10",
            Some(&cookie_a),
            None,
            None,
        )
        .await;
        assert_eq!(own_list.status(), StatusCode::OK);
        assert_eq!(response_json(own_list).await["pagination"]["total"], 1);
        let own_get = send(
            &app,
            Method::GET,
            &format!("/api/tasks/{task_id}"),
            Some(&cookie_a),
            None,
            None,
        )
        .await;
        assert_eq!(own_get.status(), StatusCode::OK);
        let own_update = send(
            &app,
            Method::PUT,
            &format!("/api/tasks/{task_id}"),
            Some(&cookie_a),
            Some(&csrf_a),
            Some(json!({"completed":true})),
        )
        .await;
        assert_eq!(own_update.status(), StatusCode::OK);

        let register_b = send(
            &app,
            Method::POST,
            "/api/auth/register",
            None,
            None,
            Some(json!({"email":email_b, "password":PASSWORD})),
        )
        .await;
        assert_eq!(register_b.status(), StatusCode::CREATED);
        let cookie_b = session_cookie(&register_b);
        let csrf_b = csrf_token(&app, &cookie_b).await;

        let list_b = send(
            &app,
            Method::GET,
            "/api/tasks?page=1&limit=10&completed=false",
            Some(&cookie_b),
            None,
            None,
        )
        .await;
        assert_eq!(list_b.status(), StatusCode::OK);
        assert_eq!(response_json(list_b).await["pagination"]["total"], 0);

        let other_get = send(
            &app,
            Method::GET,
            &format!("/api/tasks/{task_id}"),
            Some(&cookie_b),
            None,
            None,
        )
        .await;
        assert_eq!(other_get.status(), StatusCode::NOT_FOUND);
        let other_update = send(
            &app,
            Method::PUT,
            &format!("/api/tasks/{task_id}"),
            Some(&cookie_b),
            Some(&csrf_b),
            Some(json!({"title":"Hijacked"})),
        )
        .await;
        assert_eq!(other_update.status(), StatusCode::NOT_FOUND);
        let other_delete = send(
            &app,
            Method::DELETE,
            &format!("/api/tasks/{task_id}"),
            Some(&cookie_b),
            Some(&csrf_b),
            None,
        )
        .await;
        assert_eq!(other_delete.status(), StatusCode::NOT_FOUND);

        let bad_login = send(
            &app,
            Method::POST,
            "/api/auth/login",
            None,
            None,
            Some(json!({"email":email_a, "password":"IncorrectPassword123!"})),
        )
        .await;
        assert_eq!(bad_login.status(), StatusCode::UNAUTHORIZED);
        let bad_body = response_json(bad_login).await;
        assert_eq!(
            bad_body["error"]["message"],
            "Email or password is incorrect"
        );
        let login = send(
            &app,
            Method::POST,
            "/api/auth/login",
            Some(&cookie_a),
            None,
            Some(json!({"email":email_a, "password":PASSWORD})),
        )
        .await;
        assert_eq!(login.status(), StatusCode::OK);
        let rotated_cookie = session_cookie(&login);
        assert_ne!(rotated_cookie, cookie_a);
        let me = send(
            &app,
            Method::GET,
            "/api/auth/me",
            Some(&rotated_cookie),
            None,
            None,
        )
        .await;
        assert_eq!(me.status(), StatusCode::OK);

        let old_session = send(
            &app,
            Method::GET,
            "/api/auth/me",
            Some(&cookie_a),
            None,
            None,
        )
        .await;
        assert_eq!(old_session.status(), StatusCode::UNAUTHORIZED);

        let expired_token = new_token();
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, csrf_token, expires_at) VALUES ($1, $2, $3, $4, NOW() - INTERVAL '1 minute')",
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(hash_token(expired_token.as_bytes()).as_slice())
        .bind(new_token())
        .execute(&pool)
        .await
        .expect("insert an expired test session");
        let expired_cookie = format!("nexora_session={expired_token}");
        let expired = send(
            &app,
            Method::GET,
            "/api/auth/me",
            Some(&expired_cookie),
            None,
            None,
        )
        .await;
        assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);

        let csrf_rotated = csrf_token(&app, &rotated_cookie).await;
        let owner_delete = send(
            &app,
            Method::DELETE,
            &format!("/api/tasks/{task_id}"),
            Some(&rotated_cookie),
            Some(&csrf_rotated),
            None,
        )
        .await;
        assert_eq!(owner_delete.status(), StatusCode::NO_CONTENT);
        let logout = send(
            &app,
            Method::POST,
            "/api/auth/logout",
            Some(&rotated_cookie),
            Some(&csrf_rotated),
            None,
        )
        .await;
        assert_eq!(logout.status(), StatusCode::OK);
        let revoked = send(
            &app,
            Method::GET,
            "/api/auth/me",
            Some(&rotated_cookie),
            None,
            None,
        )
        .await;
        assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);

        for _ in 0..10 {
            let attempt = send_from(
                &app,
                Method::POST,
                "/api/auth/login",
                None,
                None,
                Some(json!({"email":email_a, "password":"WrongPassword123!"})),
                SocketAddr::from(([127, 0, 0, 2], 45679)),
            )
            .await;
            assert_eq!(attempt.status(), StatusCode::UNAUTHORIZED);
        }
        let rate_limited = send_from(
            &app,
            Method::POST,
            "/api/auth/login",
            None,
            None,
            Some(json!({"email":email_a, "password":PASSWORD})),
            SocketAddr::from(([127, 0, 0, 2], 45679)),
        )
        .await;
        assert_eq!(rate_limited.status(), StatusCode::TOO_MANY_REQUESTS);

        sqlx::query("DELETE FROM users WHERE email = ANY($1)")
            .bind(vec![email_a, email_b])
            .execute(&pool)
            .await
            .expect("remove only the test accounts and their dependent test rows");
        pool.close().await;
    }

    async fn send(
        app: &Router,
        method: Method,
        uri: &str,
        cookie: Option<&str>,
        csrf: Option<&str>,
        body: Option<Value>,
    ) -> Response<Body> {
        send_from(
            app,
            method,
            uri,
            cookie,
            csrf,
            body,
            SocketAddr::from(([127, 0, 0, 1], 45678)),
        )
        .await
    }

    async fn send_from(
        app: &Router,
        method: Method,
        uri: &str,
        cookie: Option<&str>,
        csrf: Option<&str>,
        body: Option<Value>,
        address: SocketAddr,
    ) -> Response<Body> {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        if let Some(csrf) = csrf {
            builder = builder.header("x-csrf-token", csrf);
        }
        if body.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        let body = body.map_or_else(Body::empty, |value| Body::from(value.to_string()));
        let mut request = builder.body(body).expect("valid integration request");
        request.extensions_mut().insert(ConnectInfo(address));
        app.clone()
            .oneshot(request)
            .await
            .expect("router handles integration request")
    }

    fn session_cookie(response: &Response<Body>) -> String {
        response
            .headers()
            .get(header::SET_COOKIE)
            .expect("session cookie is set")
            .to_str()
            .expect("valid cookie header")
            .split(';')
            .next()
            .expect("cookie pair exists")
            .to_string()
    }

    async fn csrf_token(app: &Router, cookie: &str) -> String {
        let response = send(app, Method::GET, "/api/auth/csrf", Some(cookie), None, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        response_json(response).await["csrf_token"]
            .as_str()
            .expect("csrf token")
            .to_string()
    }

    async fn response_json(response: Response<Body>) -> Value {
        let body = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("response body");
        serde_json::from_slice(&body).expect("JSON response")
    }
}
