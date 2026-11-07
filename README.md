# Nexora — Secure Three-Tier AWS Deployment

Nexora is a three-tier web application deployed on Amazon Web Services (AWS). The project demonstrates practical DevOps skills in cloud infrastructure, Linux server administration, networking, application deployment, reverse proxy configuration, database management, DNS, HTTPS, and security.

The infrastructure uses three separate Ubuntu EC2 instances across public and private subnets inside a custom AWS VPC.

**Live Application:** https://nexora.oxytumzone.com

---

## 1. Project Overview

| Category | Technology |
|---|---|
| Cloud Provider | Amazon Web Services (AWS) |
| Region | ap-southeast-2 (Sydney) |
| Operating System | Ubuntu 24.04 LTS |
| Frontend | React |
| Backend | Rust / Axum |
| Database | PostgreSQL 16 |
| Web Server | Nginx |
| Process Management | systemd |
| DNS | Amazon Route 53 / DNS A Record |
| SSL/TLS | Let's Encrypt / Certbot |
| Networking | VPC, Subnets, Route Tables, Internet Gateway, NAT Gateway |
| Security | AWS Security Groups, SSH, Private Subnets |
| Deployment | AWS Console, AWS CLI, SSH |

## 2. Architecture

Nexora follows a three-tier architecture:

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

### Tier Responsibilities

**Web Tier**
- Handles incoming HTTP and HTTPS requests.
- Serves the React frontend.
- Redirects HTTP traffic to HTTPS.
- Forwards `/api/` requests to the backend through Nginx reverse proxy.

**Application Tier**
- Runs the Rust/Axum backend.
- Handles application logic and authentication.
- Processes task-management operations.
- Communicates with the PostgreSQL database.

**Database Tier**
- Runs PostgreSQL 16.
- Stores application data.
- Accepts database connections from the Application Tier only.

## 3. AWS Infrastructure

### VPC Configuration

| Property | Value |
|---|---|
| VPC Name | nexora-vpc |
| VPC ID | `vpc-05970c4696ff8d57c` |
| IPv4 CIDR | `10.30.0.0/16` |
| AWS Region | `ap-southeast-2` |

### Subnet Configuration

| Subnet | CIDR | Availability Zone | Purpose |
|---|---|---|---|
| Public Subnet | `10.30.1.0/24` | ap-southeast-2a | Web Tier |
| App Subnet | `10.30.2.0/24` | ap-southeast-2a | Application Tier |
| Database Subnet | `10.30.3.0/24` | ap-southeast-2a | Database Tier |

The Web Tier is deployed in the public subnet. The Application and Database tiers are deployed in private subnets without public IPv4 addresses.

### Gateway and Routing

| Resource | Configuration |
|---|---|
| Internet Gateway | `nexora-igw` |
| NAT Gateway | `nexora-nat` |
| Public Route Table | Default route through Internet Gateway |
| App Route Table | Default route through NAT Gateway |
| Database Route Table | Local VPC route only |

The NAT Gateway provides outbound internet connectivity for the Application Tier while keeping the instance inaccessible directly from the public internet.

The Database subnet has no default internet route.

### EC2 Instances

| Instance | Instance ID | Instance Type | Private IP | Public IP |
|---|---|---|---|---|
| nexora-web | `i-0e9e2b2c543c235ae` | t3.micro | `10.30.1.217` | `32.236.75.43` |
| nexora-app | `i-09784a7379acaeaf6` | t3.micro | `10.30.2.82` | None |
| nexora-db | `i-06fe407fb19161724` | t3.micro | `10.30.3.195` | None |

All instances use Ubuntu 24.04 LTS.

## 4. Security Configuration

AWS Security Groups control communication between the three tiers.

| Security Group | Port | Source | Purpose |
|---|---:|---|---|
| Web SG | 80 | `0.0.0.0/0` | HTTP |
| Web SG | 443 | `0.0.0.0/0` | HTTPS |
| Web SG | 22 | Restricted admin CIDR | SSH |
| App SG | 3000 | Web Security Group | Backend API |
| App SG | 22 | Web Security Group | SSH |
| Database SG | 5432 | App Security Group | PostgreSQL |
| Database SG | 22 | App Security Group | SSH |

### Security Highlights

- Backend EC2 has no public IP address.
- Database EC2 has no public IP address.
- Backend traffic is restricted to the Web Security Group.
- PostgreSQL traffic is restricted to the App Security Group.
- SSH access is restricted through a jump-host approach.
- Application environment variables are stored outside the Git repository.
- Database connections use PostgreSQL SSL mode `require`.

**TLS note:** PostgreSQL `sslmode=require` encrypts the database connection but does not validate the server's certificate identity. Certificate identity verification remains a recommended hardening improvement.

## 5. Application Deployment

### Frontend

The React frontend is served by Nginx from:

```text
/var/www/nexora/releases/20261106-frontend-v1
```

### Backend

The Rust/Axum backend runs as a systemd service:

```text
nexora-backend.service
```

Backend listening port:

```text
3000
```

Readiness endpoint:

```text
http://127.0.0.1:3000/ready
```

### Database

PostgreSQL runs on the Database EC2 instance.

| Property | Value |
|---|---|
| Database | `nexora` |
| Application Role | `nexora_app` |
| PostgreSQL Version | 16 |
| Private IP | `10.30.3.195` |
| Port | 5432 |

The database is self-hosted on EC2; Amazon RDS is not used.

## 6. Nginx Reverse Proxy

Nginx runs on the Web Tier.

Configuration file:

```text
/etc/nginx/sites-available/nexora
```

API requests are forwarded to:

```text
http://10.30.2.82:3000
```

Request flow:

```text
Browser
   |
   | HTTPS
   v
Nginx
   |
   | /api/
   v
Rust / Axum Backend
   |
   v
PostgreSQL
```

## 7. DNS and SSL/TLS

Production domain:

**https://nexora.oxytumzone.com**

The DNS A record points to the Web EC2 Elastic IP:

```text
32.236.75.43
```

SSL/TLS is configured using Let's Encrypt and Certbot.

Verified:
- HTTPS homepage returns HTTP 200.
- HTTP requests redirect to HTTPS.
- SSL certificate validation succeeds.
- Certbot renewal dry-run succeeds.
- Automatic Certbot renewal timer is active.

## 8. Application Features

The application was tested through the production HTTPS domain.

Verified functionality:

- User registration.
- User login.
- Authentication.
- Dashboard access.
- Task creation.
- Task updating.
- Task deletion.
- API request handling.
- Unauthorized API request handling.

## 9. Verification and Testing

The following infrastructure and application components were verified:

| Component | Verification |
|---|---|
| VPC | Created and configured |
| Subnets | Public and private separation verified |
| Route Tables | Routing configuration verified |
| Internet Gateway | Public connectivity verified |
| NAT Gateway | App subnet outbound routing configured |
| Security Groups | Tier-based access restrictions configured |
| EC2 | Three instances deployed |
| Nginx | Service and reverse proxy verified |
| Backend | systemd service and readiness endpoint verified |
| PostgreSQL | Service and backend connectivity verified |
| DNS | Production hostname resolution verified |
| HTTPS | Certificate and redirect verified |
| Certbot | Renewal dry-run successful |
| Application | Authentication and task operations tested |

## 10. Screenshots and Evidence

The `screenshots/` directory contains deployment and verification evidence.

| Screenshot | Description |
|---|---|
| `01-https-homepage.png` | Production HTTPS homepage |
| `02-user-registration.png` | User registration |
| `03-user-login.png` | User login |
| `04-dashboard.png` | Application dashboard |
| `05-task-create.png` | Task creation |
| `06-task-update.png` | Task update |
| `07-task-delete.png` | Task deletion |
| `08-api-network.png` | API network requests |
| `09-https-redirect.png` | HTTP to HTTPS redirect |
| `10-ssl-certificate.png` | SSL certificate |
| `11-api-unauthorized.png` | Unauthorized API response |
| `12-vpc-overview.png` | VPC overview |
| `13-subnets.png` | Subnet configuration |
| `14a-public-route-table.png` | Public route table |
| `14b-app-route-table.png` | App route table |
| `14c-db-route-table.png` | Database route table |
| `15a-web-security-group.png` | Web Security Group |
| `15b-app-security-group.png` | App Security Group |
| `15c-db-security-group.png` | Database Security Group |
| `16-ec2-instances.png` | EC2 instances |
| `17-elastic-ip.png` | Web Elastic IP |
| `18a-database-ec2.png` | Database EC2 |
| `18b-database-storage.png` | Database storage |
| `19-nginx-status.png` | Nginx service status |
| `20-backend-status.png` | Backend service status |
| `21-postgresql-status.png` | PostgreSQL service status |
| `22-backend-health.png` | Backend readiness |
| `23-database-connection.png` | Database connectivity |
| `24-certbot-renewal.png` | Certbot renewal test |

## 11. Project Structure

```text
nexora/
├── backend/
├── frontend/
├── docs/
│   ├── ARCHITECTURE.md
│   ├── DEPLOYMENT.md
│   ├── SECURITY.md
│   └── TROUBLESHOOTING.md
├── screenshots/
├── scripts/
├── .gitignore
└── README.md
```

*The exact contents of these directories may evolve as the project develops.*

## 12. Documentation

Additional project documentation:

- [Architecture](docs/ARCHITECTURE.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [Security Documentation](docs/SECURITY.md)
- [Troubleshooting Guide](docs/TROUBLESHOOTING.md)

## 13. Important Operational Notes

- AWS resources were configured manually using AWS Console, AWS CLI, and SSH.
- Terraform and CloudFormation are not configured for this deployment.
- The database is hosted on a private EC2 instance, not RDS.
- The Database subnet has no default internet route.
- NAT Gateway and EC2 resources can incur AWS charges.
- Production environment files, passwords, private keys, and tokens must never be committed to Git.
- Review AWS billing and clean up resources when the assignment is complete.

## 14. Learning Outcomes

Through this project, the following practical DevOps skills were applied:

- AWS VPC and subnet design.
- Public and private network architecture.
- Route Tables and Gateway configuration.
- EC2 provisioning and Linux administration.
- AWS Security Group configuration.
- SSH key management and jump-host access.
- Nginx reverse proxy configuration.
- Backend deployment using systemd.
- PostgreSQL installation and configuration.
- Private database connectivity.
- DNS configuration.
- SSL/TLS certificate management.
- Certbot renewal verification.
- Application deployment and production testing.
- Technical documentation and deployment evidence.

---

**Project:** Nexora
**Focus:** Cloud Infrastructure, Linux Administration, Networking, Security, and Application Deployment
**Role Perspective:** Junior DevOps Engineer
