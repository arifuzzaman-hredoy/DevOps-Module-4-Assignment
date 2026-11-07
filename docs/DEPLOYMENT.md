# Nexora — Deployment Guide

## 1. Overview

This document describes the deployment process and production configuration of the Nexora application on Amazon Web Services (AWS).

The deployment uses a three-tier architecture:

- Web Tier: Nginx and React frontend.
- Application Tier: Rust/Axum backend.
- Database Tier: PostgreSQL 16.

The infrastructure was configured manually using AWS Console, AWS CLI, and SSH.

**Production URL:** https://nexora.oxytumzone.com

**AWS Region:** `ap-southeast-2` (Sydney)

**Operating System:** Ubuntu 24.04 LTS

> This document records the deployment architecture and operational procedures. It is not an automated provisioning script. Review the actual AWS configuration before repeating any infrastructure changes.

---

## 2. Prerequisites

Required tools and access:

- AWS account with appropriate permissions.
- AWS Console access.
- AWS CLI.
- SSH client.
- Ubuntu 24.04 LTS EC2 instances.
- Domain name with DNS management access.
- SSH private key for the EC2 instances.
- Git.

### Local Environment

The SSH private key is stored locally and must have restricted permissions.

Example:

```bash
chmod 400 ~/.ssh/nexora-key.pem
```

Never commit private keys, passwords, environment files, or database credentials to Git.

---

## 3. AWS Infrastructure Setup

### 3.1 Create VPC

Create a custom VPC with the following configuration:

| Property | Value |
|---|---|
| Name | `nexora-vpc` |
| CIDR | `10.30.0.0/16` |
| Region | `ap-southeast-2` |

### 3.2 Create Subnets

Create three subnets:

| Subnet | CIDR | Purpose |
|---|---|---|
| `nexora-public-subnet` | `10.30.1.0/24` | Web |
| `nexora-app-subnet` | `10.30.2.0/24` | Backend |
| `nexora-db-subnet` | `10.30.3.0/24` | Database |

All three subnets are in Availability Zone `ap-southeast-2a`.

Enable automatic public IPv4 assignment only for the public subnet.

### 3.3 Configure Gateways

Create and attach an Internet Gateway:

```text
nexora-igw
```

Create a NAT Gateway in the public subnet:

```text
nexora-nat
```

The NAT Gateway provides outbound internet connectivity to the private Application subnet.

### 3.4 Configure Route Tables

Configure the route tables as follows:

**Public Route Table**

| Destination | Target |
|---|---|
| `10.30.0.0/16` | Local |
| `0.0.0.0/0` | Internet Gateway |

**Application Route Table**

| Destination | Target |
|---|---|
| `10.30.0.0/16` | Local |
| `0.0.0.0/0` | NAT Gateway |

**Database Route Table**

| Destination | Target |
|---|---|
| `10.30.0.0/16` | Local |

The Database subnet has no default internet route.

---

## 4. Security Group Configuration

Create three Security Groups.

### Web Security Group

| Type | Protocol | Port | Source |
|---|---|---:|---|
| HTTP | TCP | 80 | `0.0.0.0/0` |
| HTTPS | TCP | 443 | `0.0.0.0/0` |
| SSH | TCP | 22 | Restricted administrative CIDR |

### Application Security Group

| Type | Protocol | Port | Source |
|---|---|---:|---|
| Custom TCP | TCP | 3000 | Web Security Group |
| SSH | TCP | 22 | Web Security Group |

### Database Security Group

| Type | Protocol | Port | Source |
|---|---|---:|---|
| PostgreSQL | TCP | 5432 | App Security Group |
| SSH | TCP | 22 | App Security Group |

This configuration prevents direct public access to the backend and database servers.

---

## 5. EC2 Instance Deployment

Launch three EC2 instances using Ubuntu 24.04 LTS.

| Instance | Subnet | Private IP | Public IP |
|---|---|---|---|
| `nexora-web` | Public | `10.30.1.217` | `32.236.75.43` |
| `nexora-app` | App | `10.30.2.82` | None |
| `nexora-db` | Database | `10.30.3.195` | None |

Instance type:

```text
t3.micro
```

Attach the corresponding Security Group to each instance.

The Web instance uses an Elastic IP so the production DNS record can point to a stable public address.

---

## 6. SSH Access Configuration

The Web instance acts as the SSH entry point for private instances.

Example SSH configuration:

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

### Connect to Web

```bash
ssh nexora-web
```

### Connect to Application

```bash
ssh nexora-app
```

### Connect to Database

```bash
ssh nexora-db
```

The private key must remain protected on the administrator's local machine.

---

## 7. Linux Server Preparation

Run the following commands on the appropriate Ubuntu EC2 instances.

Update package information:

```bash
sudo apt update
```

Upgrade installed packages:

```bash
sudo apt upgrade -y
```

Install commonly required utilities:

```bash
sudo apt install -y curl wget git unzip ca-certificates
```

Check operating system:

```bash
cat /etc/os-release
```

Check available disk space:

```bash
df -h
```

Check memory:

```bash
free -h
```

---

## 8. Database Deployment

The Database Tier uses PostgreSQL 16 on a private EC2 instance.

### 8.1 Install PostgreSQL

Install PostgreSQL using the configured Ubuntu package source.

```bash
sudo apt update
sudo apt install -y postgresql postgresql-contrib
```

Check service status:

```bash
sudo systemctl status postgresql
```

Enable the service at boot:

```bash
sudo systemctl enable postgresql
```

### 8.2 Database Configuration

The deployment uses:

| Property | Value |
|---|---|
| Database | `nexora` |
| Application Role | `nexora_app` |
| PostgreSQL Port | `5432` |
| Database Private IP | `10.30.3.195` |

The application role should have only the database permissions required by the application.

Database credentials must be stored securely and must never be included in this documentation.

### 8.3 Configure Network Access

PostgreSQL must listen on the required private network interface.

The PostgreSQL host-based authentication configuration must permit the Application EC2 private IP:

```text
10.30.2.82/32
```

Use SCRAM authentication for the application database role.

The Database Security Group permits TCP port `5432` only from the App Security Group.

### 8.4 Verify PostgreSQL

```bash
sudo systemctl is-active postgresql
```

Check the listening port:

```bash
sudo ss -lntp | grep 5432
```

Check PostgreSQL readiness:

```bash
sudo -u postgres pg_isready
```

---

## 9. Backend Deployment

The Application Tier runs a Rust/Axum backend as a systemd service.

### 9.1 Backend Configuration

The backend listens on:

```text
3000
```

Database server:

```text
10.30.3.195
```

Database port:

```text
5432
```

The backend environment file is stored at:

```text
/etc/nexora/backend.env
```

This file contains production configuration and sensitive values.

**Never print, copy into Git, or share the contents of this file.**

### 9.2 Environment File Permissions

The production environment file uses restricted ownership and permissions:

```text
Owner: root
Group: nexora
Mode: 640
```

Verify permissions without displaying its contents:

```bash
sudo stat -c '%U:%G %a %n' /etc/nexora/backend.env
```

### 9.3 Backend Service

The backend binary is installed at:

```text
/usr/local/bin/nexora-backend
```

The systemd service is:

```text
nexora-backend.service
```

Check service status:

```bash
sudo systemctl status nexora-backend
```

Check whether the service is active:

```bash
sudo systemctl is-active nexora-backend
```

View recent logs:

```bash
sudo journalctl -u nexora-backend -n 50 --no-pager
```

Follow logs:

```bash
sudo journalctl -u nexora-backend -f
```

Check application readiness:

```bash
curl -i http://127.0.0.1:3000/ready
```

Expected result:

```text
HTTP/1.1 200 OK
```

The readiness response should indicate that the backend and its database dependency are ready.

---

## 10. Frontend Deployment

The React frontend is served from the Web EC2 instance.

Production release directory:

```text
/var/www/nexora/releases/20261106-frontend-v1
```

Nginx serves the frontend static files.

Verify that the frontend release directory exists:

```bash
sudo ls -la /var/www/nexora/releases/
```

The Web Tier must be able to serve the application homepage over HTTPS.

---

## 11. Nginx Configuration

Nginx runs on the Web EC2 instance.

### 11.1 Install Nginx

```bash
sudo apt update
sudo apt install -y nginx
```

Enable and start Nginx:

```bash
sudo systemctl enable nginx
sudo systemctl start nginx
```

Check status:

```bash
sudo systemctl status nginx
```

### 11.2 Site Configuration

Configuration file:

```text
/etc/nginx/sites-available/nexora
```

The Nginx configuration serves the React frontend and forwards API requests to the private backend.

Backend target:

```text
http://10.30.2.82:3000
```

The `/api/` location must proxy requests to the backend without unintentionally removing the API path prefix.

### 11.3 Validate Nginx

Test the configuration:

```bash
sudo nginx -t
```

Reload Nginx after a successful configuration test:

```bash
sudo systemctl reload nginx
```

Check service status:

```bash
sudo systemctl is-active nginx
```

---

## 12. DNS Configuration

Production hostname:

```text
nexora.oxytumzone.com
```

DNS record:

| Record | Name | Value |
|---|---|---|
| A | `nexora.oxytumzone.com` | `32.236.75.43` |

The A record points to the Web EC2 Elastic IP.

Verify DNS resolution:

```bash
dig nexora.oxytumzone.com
```

Alternative:

```bash
nslookup nexora.oxytumzone.com
```

The returned address should match the Web Elastic IP.

---

## 13. SSL/TLS Configuration

The production domain uses Let's Encrypt certificates managed by Certbot.

Install Certbot and the Nginx integration:

```bash
sudo apt update
sudo apt install -y certbot python3-certbot-nginx
```

The certificate was configured for:

```text
nexora.oxytumzone.com
```

### 13.1 Verify Certificate

```bash
sudo certbot certificates
```

### 13.2 Verify Automatic Renewal

Check the Certbot timer:

```bash
systemctl status certbot.timer
```

Test renewal:

```bash
sudo certbot renew --dry-run
```

The renewal dry-run completed successfully during deployment.

### 13.3 Verify HTTPS

```bash
curl -I https://nexora.oxytumzone.com
```

Expected:

```text
HTTP/2 200
```

### 13.4 Verify HTTP Redirect

```bash
curl -I http://nexora.oxytumzone.com
```

Expected behavior:

```text
HTTP/1.1 301 Moved Permanently
```

The response should redirect to the HTTPS version of the domain.

---

## 14. Application Verification

The production application was tested through the HTTPS domain.

The following functionality was verified:

| Feature | Verification |
|---|---|
| Homepage | Successful |
| User Registration | Successful |
| User Login | Successful |
| Dashboard | Successful |
| Task Creation | Successful |
| Task Update | Successful |
| Task Deletion | Successful |
| API Requests | Successful |
| Unauthorized API Request | Expected HTTP 401 |

Authentication and task-management operations were tested through the browser.

Sensitive authentication cookies and CSRF tokens must never be included in screenshots or shared logs.

---

## 15. Production Health Checks

Run these checks on the relevant EC2 instances.

### Web Tier

```bash
sudo systemctl is-active nginx
```

```bash
sudo nginx -t
```

### Application Tier

```bash
sudo systemctl is-active nexora-backend
```

```bash
curl -i http://127.0.0.1:3000/ready
```

### Database Tier

```bash
sudo systemctl is-active postgresql
```

```bash
sudo -u postgres pg_isready
```

### Public Endpoint

```bash
curl -I https://nexora.oxytumzone.com
```

---

## 16. Deployment Evidence

Screenshots are stored in the repository's `screenshots/` directory.

Evidence includes:

- Production homepage and application functionality.
- HTTPS redirect and SSL certificate.
- VPC and subnet configuration.
- Route tables and Security Groups.
- EC2 instances and Elastic IP.
- Database instance and storage.
- Nginx and backend service status.
- PostgreSQL status and database connectivity.
- Certbot renewal verification.

Refer to the project README for the complete screenshot index.

---

## 17. Important Security Notes

- Do not expose backend or database ports publicly.
- Keep SSH access restricted to trusted administrative sources.
- Keep production environment files outside the Git repository.
- Never commit PEM files, credentials, session cookies, or API tokens.
- Use least-privilege database permissions.
- Review Security Group rules periodically.
- Monitor systemd services and application logs.
- Review AWS billing, especially NAT Gateway and EC2 costs.
- PostgreSQL `sslmode=require` encrypts database traffic but does not verify server certificate identity. Certificate verification is a recommended future improvement.

---

## 18. Cleanup and Cost Management

AWS resources can continue generating charges even when the application is not actively used.

Before removing infrastructure:

1. Confirm that all required screenshots and documentation have been saved.
2. Confirm that no further deployment verification is required.
3. Review the NAT Gateway and Elastic IP resources.
4. Review EC2 instances and storage volumes.
5. Remove resources only when they are no longer needed.

**Important:** Do not delete EC2 instances, EBS volumes, Elastic IPs, or NAT Gateway resources without checking their dependencies and confirming that the evidence and application data are backed up.

---

## 19. Deployment Summary

The Nexora production deployment demonstrates:

- Custom AWS VPC design.
- Public and private subnet separation.
- Internet Gateway and NAT Gateway configuration.
- Three-tier EC2 deployment.
- Security Group-based network isolation.
- SSH jump-host access.
- PostgreSQL database administration.
- Rust/Axum backend deployment with systemd.
- Nginx reverse proxy configuration.
- React frontend hosting.
- DNS configuration.
- HTTPS certificate management.
- Production application verification.
- Technical documentation and deployment evidence.

---

**Project:** Nexora
**Deployment Model:** Three-Tier AWS Architecture
**Cloud Provider:** Amazon Web Services
**Region:** `ap-southeast-2`
**Operating System:** Ubuntu 24.04 LTS
