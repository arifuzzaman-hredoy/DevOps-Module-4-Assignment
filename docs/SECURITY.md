# Nexora — Security Documentation

## 1. Overview

Security is an important part of the Nexora production infrastructure.

The project uses a three-tier AWS architecture to separate public-facing services from private application and database services.

The security design focuses on:

- Network isolation.
- Least-privilege access.
- SSH access control.
- Database access restrictions.
- Secure environment configuration.
- HTTPS and TLS encryption.
- Linux service security.
- Protection of application credentials.

**Production Domain:** `nexora.oxytumzone.com`

**AWS Region:** `ap-southeast-2`

---

## 2. Network Security Architecture

The Nexora infrastructure is deployed inside a custom AWS VPC.

VPC configuration:

| Property | Value |
|---|---|
| VPC Name | `nexora-vpc` |
| CIDR | `10.30.0.0/16` |
| Region | `ap-southeast-2` |

The network is divided into three subnets.

| Subnet | CIDR | Security Purpose |
|---|---|---|
| Public | `10.30.1.0/24` | Public Web Tier |
| Application | `10.30.2.0/24` | Private Backend |
| Database | `10.30.3.0/24` | Private PostgreSQL |

### Network Isolation

- Only the Web Tier has a public IP address.
- The Application Tier has no public IP address.
- The Database Tier has no public IP address.
- The Database subnet has no default internet route.
- Private services communicate through the VPC network.

This reduces direct exposure of internal application and database services to the public internet.

---

## 3. AWS Security Groups

Security Groups control inbound access to each EC2 instance.

### 3.1 Web Security Group

| Protocol | Port | Source | Purpose |
|---|---:|---|---|
| TCP | 80 | `0.0.0.0/0` | HTTP |
| TCP | 443 | `0.0.0.0/0` | HTTPS |
| TCP | 22 | Restricted administrative CIDR | SSH |

HTTP and HTTPS are publicly accessible because users need to access the website.

SSH access is restricted to the configured administrative CIDR.

### 3.2 Application Security Group

| Protocol | Port | Source | Purpose |
|---|---:|---|---|
| TCP | 3000 | Web Security Group | Backend API |
| TCP | 22 | Web Security Group | SSH administration |

The backend port is not publicly accessible.

Only the Web Security Group can initiate backend application connections.

### 3.3 Database Security Group

| Protocol | Port | Source | Purpose |
|---|---:|---|---|
| TCP | 5432 | App Security Group | PostgreSQL |
| TCP | 22 | App Security Group | SSH administration |

PostgreSQL is not exposed to the public internet.

Only the Application Security Group can initiate PostgreSQL connections.

---

## 4. SSH Access Security

SSH access is controlled through Security Group rules and a jump-host architecture.

Connection path:

```text
Administrator
     |
     | SSH
     v
 Web EC2
     |
     | SSH / ProxyJump
     v
 App EC2
     |
     | SSH / ProxyJump
     v
 Database EC2
```

### SSH Configuration

The administrator uses SSH aliases and ProxyJump to access private instances.

Example:

```sshconfig
Host nexora-web
    HostName 32.236.75.43
    User ubuntu
    IdentityFile ~/.ssh/nexora-key.pem
    IdentitiesOnly yes

Host nexora-app
    HostName 10.30.2.82
    User ubuntu
    IdentityFile ~/.ssh/nexora-key.pem
    IdentitiesOnly yes
    ProxyJump nexora-web

Host nexora-db
    HostName 10.30.3.195
    User ubuntu
    IdentityFile ~/.ssh/nexora-key.pem
    IdentitiesOnly yes
    ProxyJump nexora-app
```

### SSH Key Protection

The private SSH key must:

- Remain on the administrator's trusted machine.
- Have restricted file permissions.
- Never be uploaded to GitHub.
- Never be shared publicly.
- Never be included in screenshots or documentation.

Example permission command:

```bash
chmod 400 ~/.ssh/nexora-key.pem
```

---

## 5. Database Security

The database runs on a private EC2 instance using PostgreSQL 16.

Database configuration:

| Property | Value |
|---|---|
| Database | `nexora` |
| Application Role | `nexora_app` |
| Host | `10.30.3.195` |
| Port | `5432` |

### Database Access Restrictions

- PostgreSQL is not publicly accessible.
- Database EC2 has no public IP address.
- Database Security Group permits TCP 5432 only from the App Security Group.
- PostgreSQL host-based authentication restricts application access to the backend private IP.
- SCRAM authentication is used for the application database role.

### Database Connection Encryption

The backend uses PostgreSQL SSL mode:

```text
sslmode=require
```

This encrypts the database connection.

**Important limitation:** `sslmode=require` does not validate the database server's certificate identity.

A recommended future improvement is to configure certificate-based server identity verification using `sslmode=verify-full` with a trusted CA certificate.

---

## 6. Environment Variables and Secrets Management

Production configuration is stored outside the Git repository.

Backend environment file:

```text
/etc/nexora/backend.env
```

The file contains sensitive production configuration.

### File Permissions

The deployment uses:

| Property | Value |
|---|---|
| Owner | `root` |
| Group | `nexora` |
| Permission | `640` |

Verify permissions without displaying the file contents:

```bash
sudo stat -c '%U:%G %a %n' /etc/nexora/backend.env
```

### Secret Protection Rules

Never commit or publicly share:

- Database passwords.
- Application secret keys.
- Session signing secrets.
- Production environment files.
- SSH private keys.
- Authentication cookies.
- CSRF tokens.
- API tokens.

The `.gitignore` file should exclude local environment files and private credentials.

Example patterns:

```gitignore
.env
.env.*
!.env.example
*.pem
*.key
```

Review these patterns against the actual project requirements before modifying `.gitignore`.

### Recommended Improvement

For future production deployments, AWS Secrets Manager or AWS Systems Manager Parameter Store can be considered for centralized secret management.

---

## 7. HTTPS and TLS Security

The production application uses HTTPS.

Domain:

```text
nexora.oxytumzone.com
```

TLS configuration:

| Component | Technology |
|---|---|
| Web Server | Nginx |
| Certificate Authority | Let's Encrypt |
| Certificate Management | Certbot |
| HTTPS Port | 443 |
| HTTP Port | 80 |

### HTTP Redirect

HTTP requests are redirected to HTTPS.

Verify:

```bash
curl -I http://nexora.oxytumzone.com
```

Expected behavior:

```text
HTTP/1.1 301 Moved Permanently
```

### HTTPS Verification

```bash
curl -I https://nexora.oxytumzone.com
```

Expected:

```text
HTTP/2 200
```

### Certificate Verification

```bash
sudo certbot certificates
```

### Automatic Renewal

Check the Certbot timer:

```bash
systemctl status certbot.timer
```

Test renewal:

```bash
sudo certbot renew --dry-run
```

The renewal dry-run was completed successfully during deployment.

---

## 8. Application Security

The application provides authentication and task-management functionality.

Verified application behavior includes:

- User registration.
- User login.
- Authenticated dashboard access.
- Task creation.
- Task updating.
- Task deletion.
- Unauthorized API request handling.

### Authentication

The backend provides authentication endpoints, including:

```text
/api/auth/register
/api/auth/login
/api/auth/me
/api/auth/csrf
/api/auth/logout
```

The production application uses secure session-cookie configuration.

Authentication cookies and CSRF tokens must not be exposed in screenshots, logs, or public documentation.

### Unauthorized Access

An unauthenticated request to a protected endpoint returns HTTP 401 as expected.

This confirms that protected API functionality requires authentication.

---

## 9. Linux Service Security

The backend runs as a systemd service:

```text
nexora-backend.service
```

The production binary is installed at:

```text
/usr/local/bin/nexora-backend
```

Check service status:

```bash
sudo systemctl status nexora-backend
```

View recent logs:

```bash
sudo journalctl -u nexora-backend -n 50 --no-pager
```

The Web Tier uses Nginx as its public-facing web server.

Check Nginx:

```bash
sudo systemctl status nginx
```

Check PostgreSQL:

```bash
sudo systemctl status postgresql
```

Only required services and network ports should be exposed.

---

## 10. Security Verification Checklist

| Security Control | Status |
|---|---|
| Custom VPC configured | Verified |
| Public/private subnet separation | Verified |
| Backend has no public IP | Verified |
| Database has no public IP | Verified |
| Backend port restricted by Security Group | Verified |
| PostgreSQL port restricted by Security Group | Verified |
| SSH restricted through jump-host architecture | Configured |
| Production environment file outside Git | Configured |
| Environment file permissions restricted | Configured |
| HTTPS enabled | Verified |
| HTTP to HTTPS redirect | Verified |
| Let's Encrypt certificate | Verified |
| Certbot renewal dry-run | Successful |
| PostgreSQL connection encryption | Configured |
| PostgreSQL certificate identity verification | Future improvement |

---

## 11. Security Hardening Recommendations

The following improvements can strengthen the production environment further.

### 11.1 Infrastructure as Code

Use Terraform to define and version-control:

- VPC.
- Subnets.
- Route Tables.
- Internet Gateway.
- NAT Gateway.
- Security Groups.
- EC2 instances.

### 11.2 Centralized Secrets Management

Use AWS Secrets Manager or Systems Manager Parameter Store for sensitive application configuration.

### 11.3 Database TLS Verification

Configure PostgreSQL certificate verification so the backend validates the database server identity.

### 11.4 Monitoring and Logging

Consider:

- Prometheus.
- Grafana.
- Loki.
- AWS CloudWatch.

### 11.5 Automated Security Scanning

Integrate tools such as:

- Trivy.
- Dependabot.
- GitHub secret scanning.
- Dependency vulnerability scanning.

### 11.6 Backup and Recovery

Implement:

- Automated database backups.
- EBS snapshot policies.
- Backup retention.
- Recovery testing.

### 11.7 Access Management

Use IAM least-privilege policies and avoid unnecessary administrative permissions.

Review Security Group rules and SSH access sources periodically.

---

## 12. Security Incident Response Basics

If a security issue is suspected:

1. Identify the affected EC2 instance or application component.
2. Review relevant system and application logs.
3. Restrict suspicious network access through Security Groups.
4. Rotate affected credentials and secrets.
5. Check for unauthorized system or application changes.
6. Restore from a verified backup if required.
7. Document the incident and corrective actions.

Do not delete evidence or terminate production resources before assessing the incident and preserving required data.

---

## 13. Security Summary

Nexora uses a layered security approach based on:

- AWS network isolation.
- Public and private subnet separation.
- Security Group-based access control.
- SSH jump-host administration.
- Private PostgreSQL access.
- Restricted environment-file permissions.
- HTTPS and Let's Encrypt.
- Application authentication.
- Production service monitoring.

The deployment demonstrates practical cloud security and infrastructure-hardening techniques for a junior DevOps engineering project.

---

**Project:** Nexora
**Security Model:** Defense in Depth
**Cloud Provider:** Amazon Web Services
**Region:** `ap-southeast-2`
