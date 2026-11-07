# Nexora — Troubleshooting Guide

## 1. Overview

This document describes common infrastructure, networking, deployment, and application issues that may occur while operating the Nexora production environment.

It includes diagnostic commands, possible causes, and recommended solutions.

**Project:** Nexora
**Domain:** `nexora.oxytumzone.com`
**AWS Region:** `ap-southeast-2`
**Operating System:** Ubuntu 24.04 LTS

---

## 2. SSH Connection Problems

### Problem: SSH Connection Timeout

Example:

```text
ssh: connect to host <IP> port 22: Connection timed out
```

Possible causes:

- SSH inbound rule does not include the administrator's current public IP.
- The EC2 instance is stopped.
- The instance is in an incorrect subnet.
- The route table or network configuration is incorrect.
- Network connectivity is unavailable.

Diagnostic commands:

```bash
aws ec2 describe-instances \
  --region ap-southeast-2 \
  --instance-ids <INSTANCE_ID>
```

Check the Web Security Group's SSH inbound rule.

If the administrator's public IP changes, update the restricted SSH source CIDR with the correct trusted address.

Do not expose SSH to `0.0.0.0/0` as a permanent workaround.

### Problem: Permission Denied (Publickey)

Example:

```text
Permission denied (publickey).
```

Possible causes:

- Incorrect private key.
- Incorrect username.
- Incorrect SSH configuration.
- Incorrect private-key permissions.

Check key permissions:

```bash
chmod 400 ~/.ssh/nexora-key.pem
```

For Ubuntu EC2 instances, the default SSH username is:

```text
ubuntu
```

Test the connection:

```bash
ssh nexora-web
```

### Problem: Private EC2 Instance Is Not Reachable

The App and Database instances do not have public IP addresses.

Use the configured SSH ProxyJump chain:

```bash
ssh nexora-app
```

```bash
ssh nexora-db
```

Verify that:

- The Web instance is running.
- The App Security Group permits SSH from the Web Security Group.
- The Database Security Group permits SSH from the App Security Group.
- The SSH aliases and private IP addresses are correct.

---

## 3. Application Connectivity Problems

### Problem: Backend API Is Unavailable

Example:

```text
502 Bad Gateway
```

Possible causes:

- Backend systemd service is stopped.
- Backend process failed to start.
- Nginx cannot reach the backend.
- Backend is listening on an unexpected interface or port.

On the App EC2 instance:

```bash
sudo systemctl status nexora-backend
```

Check recent backend logs:

```bash
sudo journalctl -u nexora-backend -n 50 --no-pager
```

Check the listening port:

```bash
sudo ss -lntp | grep ':3000'
```

Check the backend health endpoint from the App instance:

```bash
curl -i http://127.0.0.1:3000/ready
```

If the service is stopped, investigate the logs before restarting it.

After identifying and correcting the issue:

```bash
sudo systemctl restart nexora-backend
```

### Problem: Nginx Returns 502 Bad Gateway

Check Nginx status:

```bash
sudo systemctl status nginx
```

Validate Nginx configuration:

```bash
sudo nginx -t
```

Review error logs:

```bash
sudo tail -n 50 /var/log/nginx/error.log
```

Verify that the backend private IP and port in the Nginx configuration are correct.

The production API upstream is:

```text
http://10.30.2.82:3000
```

Also verify that the Web Security Group permits the required backend connection.

---

## 4. Database Connectivity Problems

### Problem: Backend Cannot Connect to PostgreSQL

Possible causes:

- PostgreSQL service is stopped.
- Incorrect database hostname or port.
- Database Security Group does not permit the App Security Group.
- Incorrect database credentials.
- Incorrect PostgreSQL host-based authentication rules.
- PostgreSQL is not listening on the expected private interface.

On the Database EC2 instance:

```bash
sudo systemctl status postgresql
```

Check PostgreSQL listening port:

```bash
sudo ss -lntp | grep ':5432'
```

Check PostgreSQL logs:

```bash
sudo journalctl -u postgresql -n 50 --no-pager
```

Check database readiness:

```bash
pg_isready -h 10.30.3.195 -p 5432
```

On the App EC2 instance, check network connectivity:

```bash
nc -vz 10.30.3.195 5432
```

A successful TCP connection confirms network-level reachability, but does not by itself prove database authentication works.

### Problem: Database Authentication Failed

Example:

```text
password authentication failed
```

Check:

- Application database username.
- Database name.
- Secret configuration.
- PostgreSQL role permissions.
- `pg_hba.conf` authentication rules.

Never print or share the production environment file.

Do not use commands that expose database passwords or other secrets in terminal output or screenshots.

---

## 5. DNS Problems

### Problem: Domain Does Not Resolve

Check DNS resolution:

```bash
dig nexora.oxytumzone.com
```

Or:

```bash
nslookup nexora.oxytumzone.com
```

Verify that the domain's A record points to the Web EC2 Elastic IP:

```text
32.236.75.43
```

If DNS records were recently changed, allow time for DNS propagation according to the record's TTL.

### Problem: Domain Points to an Old IP

The Web EC2 instance uses an Elastic IP to maintain a stable public address.

Verify the Elastic IP association in the AWS EC2 console.

If the Elastic IP association changes, update the DNS record accordingly.

---

## 6. HTTPS and SSL Problems

### Problem: SSL Certificate Error

Check installed certificates:

```bash
sudo certbot certificates
```

Check Nginx configuration:

```bash
sudo nginx -t
```

Check HTTPS response:

```bash
curl -Iv https://nexora.oxytumzone.com
```

Verify:

- The certificate covers the correct domain.
- The certificate has not expired.
- Nginx references the correct certificate paths.
- TCP port 443 is allowed by the Web Security Group.

### Problem: HTTP Does Not Redirect to HTTPS

Check the HTTP response:

```bash
curl -I http://nexora.oxytumzone.com
```

Expected behavior:

```text
HTTP/1.1 301 Moved Permanently
```

Review the Nginx server block and verify that HTTP requests are redirected to HTTPS.

### Problem: Certbot Renewal Fails

Check the renewal timer:

```bash
systemctl status certbot.timer
```

Test renewal:

```bash
sudo certbot renew --dry-run
```

Review Certbot logs:

```bash
sudo tail -n 50 /var/log/letsencrypt/letsencrypt.log
```

Confirm that the domain resolves correctly and that the required HTTP validation path is reachable.

---

## 7. Authentication Problems

### Problem: Login or Registration Fails

Possible causes:

- Backend service is unavailable.
- Database connection failed.
- Invalid user credentials.
- Request validation failed.
- Browser cookie configuration is incorrect.

Check the browser's Network tab and inspect the HTTP response status.

Review backend logs without exposing credentials, cookies, or tokens.

### Problem: Protected API Returns HTTP 401

An HTTP 401 response from a protected endpoint can be expected when the request does not contain a valid authenticated session.

Verify that:

- The user is logged in.
- The browser is sending the required session cookie.
- The session has not expired.
- The request is sent over HTTPS.

Do not share authentication cookies or session values when reporting an issue.

### Problem: CSRF Validation Fails

Check that the frontend follows the application's CSRF token workflow.

Verify that:

- The required CSRF token is obtained.
- The expected request header is sent.
- The browser includes the required session cookie.
- The request is made to the correct API endpoint.

Never publish CSRF tokens in screenshots or logs.

---

## 8. Linux Service and Resource Problems

### Problem: Backend Service Keeps Restarting

Check:

```bash
sudo systemctl status nexora-backend
```

Review logs:

```bash
sudo journalctl -u nexora-backend -n 100 --no-pager
```

Check available memory:

```bash
free -h
```

Check disk space:

```bash
df -h
```

Check running processes:

```bash
top
```

Possible causes include:

- Invalid environment configuration.
- Database connection failure.
- Application startup error.
- Insufficient memory.
- Incorrect file permissions.

Investigate the root cause before repeatedly restarting the service.

### Problem: Disk Space Is Running Low

Check filesystem usage:

```bash
df -h
```

Check directory sizes:

```bash
sudo du -h -d 1 /var/log
```

Review large log files and application artifacts before removing anything.

Do not delete production database files, application data, or Docker volumes without a verified backup and a clear recovery plan.

---

## 9. AWS Networking Problems

### Problem: App Tier Cannot Reach the Internet

The App subnet uses the NAT Gateway for outbound internet connectivity.

Verify:

- NAT Gateway state is `Available`.
- App route table has the correct default route.
- NAT Gateway is located in the public subnet.
- Public route table has a route to the Internet Gateway.
- Network ACLs and Security Groups permit the required traffic.

The Database subnet is intentionally configured without a default internet route.

### Problem: Web Tier Cannot Reach App Tier

Check:

- Web and App instances are in the correct VPC.
- App Security Group allows TCP port 3000 from the Web Security Group.
- Backend is listening on the expected port.
- The Nginx upstream address is correct.

Test connectivity from the Web instance:

```bash
nc -vz 10.30.2.82 3000
```

---

## 10. Safe Troubleshooting Procedure

Follow this order when investigating production problems:

1. Identify the affected service or infrastructure component.
2. Check the relevant service status.
3. Review logs and error messages.
4. Verify network connectivity.
5. Check Security Group and routing configuration.
6. Inspect configuration without exposing secrets.
7. Make the smallest necessary correction.
8. Verify the service and application behavior.
9. Document the root cause and resolution.

Avoid unnecessary changes to working production infrastructure.

Do not delete resources, volumes, or database data as an initial troubleshooting step.

---

## 11. Useful Diagnostic Commands

| Purpose | Command |
|---|---|
| Backend status | `sudo systemctl status nexora-backend` |
| Nginx status | `sudo systemctl status nginx` |
| PostgreSQL status | `sudo systemctl status postgresql` |
| Backend logs | `sudo journalctl -u nexora-backend -n 50 --no-pager` |
| Nginx configuration test | `sudo nginx -t` |
| Nginx error logs | `sudo tail -n 50 /var/log/nginx/error.log` |
| Listening ports | `sudo ss -lntp` |
| Memory usage | `free -h` |
| Disk usage | `df -h` |
| DNS lookup | `dig nexora.oxytumzone.com` |
| HTTPS check | `curl -I https://nexora.oxytumzone.com` |
| Database readiness | `pg_isready -h 10.30.3.195 -p 5432` |

---

## 12. Final Notes

This guide documents practical troubleshooting procedures for the Nexora three-tier AWS deployment.

The goal is to diagnose problems systematically, protect production data, and restore services without introducing unnecessary infrastructure changes.

**Project:** Nexora
**Environment:** Production
**Cloud Provider:** Amazon Web Services
**Region:** `ap-southeast-2`
