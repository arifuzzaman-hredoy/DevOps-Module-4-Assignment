# Nexora — System Architecture

## 1. Overview

Nexora is a three-tier web application deployed on Amazon Web Services (AWS).

The architecture separates the infrastructure into three layers:

1. Web Tier — Public-facing frontend and reverse proxy.
2. Application Tier — Private backend application server.
3. Database Tier — Private PostgreSQL database server.

Each tier runs on a separate Ubuntu 24.04 LTS EC2 instance inside a custom AWS VPC.

The infrastructure was manually configured using AWS Console, AWS CLI, and SSH. Terraform and CloudFormation are not configured for this deployment.

## 2. High-Level Architecture

```text
                         INTERNET
                            |
                            |
                  nexora.oxytumzone.com
                            |
                       HTTPS / 443
                            |
                    Elastic IP
                    32.236.75.43
                            |
                  +-------------------+
                  |      WEB TIER     |
                  |                   |
                  |    Ubuntu EC2     |
                  |       Nginx       |
                  |   React Frontend  |
                  |                   |
                  |   10.30.1.217     |
                  +-------------------+
                            |
                     /api/ Reverse Proxy
                         TCP 3000
                            |
                  +-------------------+
                  |      APP TIER     |
                  |                   |
                  |    Ubuntu EC2     |
                  |    Rust / Axum    |
                  |                   |
                  |    10.30.2.82     |
                  +-------------------+
                            |
                     PostgreSQL 5432
                            |
                  +-------------------+
                  |   DATABASE TIER   |
                  |                   |
                  |    Ubuntu EC2     |
                  |   PostgreSQL 16   |
                  |                   |
                  |    10.30.3.195    |
                  +-------------------+
```

## 3. AWS Network Architecture

### 3.1 VPC Configuration

| Property | Value |
|---|---|
| Name | `nexora-vpc` |
| VPC ID | `vpc-05970c4696ff8d57c` |
| IPv4 CIDR | `10.30.0.0/16` |
| AWS Region | `ap-southeast-2` |
| Availability Zone | `ap-southeast-2a` |

The VPC provides an isolated network environment for the Nexora infrastructure.

### 3.2 Subnet Configuration

| Subnet | CIDR | Purpose | Public IP |
|---|---|---|---|
| Public Subnet | `10.30.1.0/24` | Web Tier | Enabled |
| App Subnet | `10.30.2.0/24` | Application Tier | Disabled |
| Database Subnet | `10.30.3.0/24` | Database Tier | Disabled |

All three subnets are located in `ap-southeast-2a`.

The Web Tier is deployed in the public subnet. The Application and Database tiers are deployed in private subnets.

### 3.3 Internet Gateway and NAT Gateway

| Resource | Name | Purpose |
|---|---|---|
| Internet Gateway | `nexora-igw` | Public internet connectivity |
| NAT Gateway | `nexora-nat` | Outbound internet access for private App subnet |

The Internet Gateway is attached to the Nexora VPC.

The NAT Gateway allows resources in the App subnet to initiate outbound internet connections without assigning a public IP address to the backend instance.

The Database subnet does not have a default internet route.

### 3.4 Route Table Configuration

| Route Table | Associated Subnet | Default Route |
|---|---|---|
| Public Route Table | Public Subnet | Internet Gateway |
| App Route Table | App Subnet | NAT Gateway |
| Database Route Table | Database Subnet | No default internet route |

All subnets retain the VPC local route for internal communication.

## 4. EC2 Infrastructure

Three Ubuntu 24.04 LTS EC2 instances host the application.

| Instance | Instance ID | Instance Type | Private IP | Public IP |
|---|---|---|---|---|
| `nexora-web` | `i-0e9e2b2c543c235ae` | `t3.micro` | `10.30.1.217` | `32.236.75.43` |
| `nexora-app` | `i-09784a7379acaeaf6` | `t3.micro` | `10.30.2.82` | None |
| `nexora-db` | `i-06fe407fb19161724` | `t3.micro` | `10.30.3.195` | None |

### 4.1 Web Tier

**Instance:** `nexora-web`

Responsibilities:

- Accept public HTTP and HTTPS requests.
- Serve the React frontend.
- Redirect HTTP traffic to HTTPS.
- Forward `/api/` requests to the backend.
- Terminate HTTPS connections using Nginx.

Frontend deployment directory:

```text
/var/www/nexora/releases/20261106-frontend-v1
```

Nginx configuration:

```text
/etc/nginx/sites-available/nexora
```

Backend proxy target:

```text
http://10.30.2.82:3000
```

### 4.2 Application Tier

**Instance:** `nexora-app`

Responsibilities:

- Run the Rust/Axum backend.
- Process application requests.
- Handle user authentication.
- Perform task-management operations.
- Communicate with PostgreSQL.
- Apply database migrations.
- Provide application readiness status.

Systemd service:

```text
nexora-backend.service
```

Backend port:

```text
3000
```

Readiness endpoint:

```text
http://127.0.0.1:3000/ready
```

### 4.3 Database Tier

**Instance:** `nexora-db`

Responsibilities:

- Store application user records.
- Store task data.
- Store authentication session data.
- Provide persistent database services to the backend.

Database configuration:

| Property | Value |
|---|---|
| Database Engine | PostgreSQL |
| Version | 16 |
| Database Name | `nexora` |
| Application Role | `nexora_app` |
| Private IP | `10.30.3.195` |
| Port | `5432` |

The database is hosted on a private EC2 instance. Amazon RDS is not used.

## 5. Security Group Architecture

Security Groups control traffic between the three infrastructure layers.

| Security Group | Port | Source | Purpose |
|---|---:|---|---|
| Web SG | 80 | `0.0.0.0/0` | HTTP |
| Web SG | 443 | `0.0.0.0/0` | HTTPS |
| Web SG | 22 | Restricted admin CIDR | SSH |
| App SG | 3000 | Web Security Group | Backend API |
| App SG | 22 | Web Security Group | SSH |
| Database SG | 5432 | App Security Group | PostgreSQL |
| Database SG | 22 | App Security Group | SSH |

### Security Design

- Only the Web Tier accepts public HTTP and HTTPS traffic.
- The Application Tier has no public IP address.
- The Database Tier has no public IP address.
- Backend port `3000` is accessible from the Web Security Group.
- PostgreSQL port `5432` is accessible from the App Security Group.
- SSH access to private instances follows a jump-host path.

## 6. SSH Access Architecture

Private EC2 instances are accessed through SSH ProxyJump.

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

SSH aliases:

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

The private key must remain secure and must never be committed to the Git repository.

## 7. Application Request Flow

### 7.1 Frontend Request

1. A user opens `https://nexora.oxytumzone.com`.
2. DNS resolves the hostname to the Web EC2 Elastic IP.
3. Nginx receives the HTTPS request.
4. Nginx serves the React frontend.

### 7.2 API Request

1. The frontend sends a request to an `/api/` endpoint.
2. Nginx forwards the request to `10.30.2.82:3000`.
3. The Rust/Axum backend processes the request.
4. The backend communicates with PostgreSQL at `10.30.3.195:5432`.
5. PostgreSQL returns the requested data.
6. The backend sends the response to Nginx.
7. Nginx returns the response to the browser.

## 8. DNS and HTTPS Architecture

Production domain:

```text
nexora.oxytumzone.com
```

DNS configuration:

| Record Type | Hostname | Value |
|---|---|---|
| A | `nexora.oxytumzone.com` | `32.236.75.43` |

TLS configuration:

- Certificate Authority: Let's Encrypt
- Certificate management: Certbot
- HTTPS termination: Nginx
- HTTP to HTTPS redirect: Enabled
- Automatic renewal: Certbot systemd timer

The certificate renewal dry-run was tested successfully.

## 9. Database Connectivity and Encryption

The backend connects to PostgreSQL through the private VPC network.

Database connection details:

```text
Backend: 10.30.2.82
Database: 10.30.3.195
Port: 5432
```

PostgreSQL access is restricted through the Database Security Group and PostgreSQL host-based authentication configuration.

The backend uses PostgreSQL SSL mode `require`.

**Important security consideration:** `sslmode=require` encrypts the database connection but does not verify the database server's certificate identity. Using certificate verification, such as `verify-full`, is a recommended future hardening improvement.

## 10. Infrastructure Verification

The following components were configured and tested during deployment:

| Component | Verification |
|---|---|
| VPC | Created and configured |
| Subnets | Public/private separation verified |
| Internet Gateway | Public connectivity verified |
| NAT Gateway | App subnet outbound routing configured |
| Route Tables | Routing configuration verified |
| Security Groups | Tier-based access restrictions verified |
| EC2 | Three instances deployed |
| Nginx | Service and reverse proxy verified |
| Backend | systemd service and readiness endpoint verified |
| PostgreSQL | Service and backend connectivity verified |
| DNS | Production hostname resolution verified |
| HTTPS | Certificate and redirect verified |
| Certbot | Renewal dry-run successful |
| Application | Registration, login, and task operations tested |

Screenshots and deployment evidence are available in the `screenshots/` directory.

## 11. Operational Considerations

- NAT Gateway incurs hourly and data-processing charges.
- EC2 instances incur charges while running.
- The Database subnet has no default internet route.
- Infrastructure was configured manually; Terraform and CloudFormation are not configured.
- Production environment variables and credentials must remain outside version control.
- Private SSH keys must never be shared or committed.
- AWS billing should be reviewed and unused resources should be cleaned up after the assignment.

## 12. Future Improvements

Potential improvements include:

- Infrastructure as Code using Terraform.
- Automated CI/CD deployment.
- Centralized monitoring and alerting.
- Centralized log management.
- Automated database backups and recovery testing.
- PostgreSQL certificate identity verification.
- Multi-AZ high availability.
- Managed database deployment using Amazon RDS.
- Improved secret management using AWS Secrets Manager.

---

**Project:** Nexora
**Architecture:** Three-Tier AWS Deployment
**Cloud Provider:** Amazon Web Services
**Operating System:** Ubuntu 24.04 LTS
