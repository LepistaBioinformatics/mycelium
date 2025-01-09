# Mycelium API Gateway

**Mycelium** is a powerful **API Gateway** designed to provide secure and
flexible management of API services. It combines advanced authentication
mechanisms with robust resource and access control, making it the ideal solution
for organizations managing multiple clients and services.

---

## 🚀 Key Features

### 🌐 **API Gateway with Multi-Tenant Support**

- Mycelium functions as a central **API Gateway**, streamlining the management
  of API requests and responses.
- It supports **tenants**, which are namespaces that separate resources for
  different user groups. This feature is crucial for businesses offering APIs to
  multiple clients, ensuring resource isolation and secure operations.

### 🔐 **Comprehensive Authentication Options**

Mycelium provides a variety of authentication methods to suit different
scenarios:

- **Native Authentication**: Standard username and password login for end users
  with TOTP support.
- **OAuth2 Integration**: Authentication using Azure and Google accounts for
  seamless third-party integration.
- **Connection String Authentication**: Token-based authentication for
  service-to-service communication, with tokens generated securely by Mycelium.

### 🛡️ **Role Management System**

- Mycelium includes a full-featured **role management system**, allowing users
  to create and manage hierarchical roles.
- This system enables flexible role definitions, ensuring that access levels
  align with specific organizational requirements.

### 🎛️ **Permission Management System**

- Mycelium provides granular **permission management**, letting users define
  specific access rights for various resources.
- This ensures fine-grained control over who can view, modify, or manage APIs
  and related resources.

### 🪪 **Detailed Access Control**

- Downstream APIs can receive user profile information from Mycelium, allowing
  them to customize responses based on the user's role and permissions.

### **Endpoints protection by role**

- Downstream APIs can be protected by role and specific permissions, ensuring
  that only users with the appropriate access level can interact with them.
- The filtration process is performed at the gateway level, preventing
  unauthorized access to downstream services.

---

By combining the capabilities of an API Gateway with advanced authentication,
multi-tenant support, and detailed access control systems, Mycelium empowers
organizations to securely manage their APIs and scale efficiently across diverse
client needs.

## 🛡️ Mycelium roles

Mycelium works with a set of predefined roles that can be assigned to users.
Predefined roles ensures a consistent and secure access control across the
organization. Standard roles include full application access roles (super user
[SU] and not super user roles), as well as roles with limited access to specific
tenants or resources.

### **Super User (SU) Roles**

Super users has the ability to scale up and down user and permissions and
perform application level operations. Super users inherits abilities from all
other roles.

- **Staff's**: Has the exclusive ability to upgrade and downgrade new users to
  the manager's and staff's roles.

- **Manager's**: Has the exclusive ability to manage tenants and their
  resources.

### **Application Level Roles (not SU)**

- TODO

### **Tenant Level Roles**

- TODO

### **Account Level Roles**

- TODO

### **Beginners**

- TODO

### **Service**

- TODO

---
