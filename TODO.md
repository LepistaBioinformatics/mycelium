# Mycelium TODO checklist.
Todo elements are shown by the application layer, following:

## System rules
* Domain: Here only application core definitions resides. Like the structs for 
the data transfer layer, structural definitions of entities, and some utils 
shared along the application operations, like error handlers.
* UseCases: Here granular and more specific rules exists. All actors operations 
are mirrored here.

## I/O layers
* Adapters: A layer of outbound elements.
* Ports: A layer of inbound elements.

___

## Domain - DTOs
- [ ] Create profile DTO.

## Domain - Entities

### Staff
- [ ] Create entities for applications registration.
- [ ] Create entities for applications deletion.
- [ ] Create entities for applications updating.
- [ ] Create entities for applications fetching.

### Managers
- [ ] Create entities for role registration.
- [ ] Create entities for role deletion.
- [ ] Create entities for role updating.
- [ ] Create entities for role fetching.
- [ ] Create entities for account-type registration.
- [ ] Create entities for account-type deletion.
- [ ] Create entities for account-type updating.
- [ ] Create entities for account-type fetching.
- [ ] Create entity to deny account.
- [ ] Create entity to admit account.

### DefaultUsers
- [ ] Create entities for account registration.
- [ ] Create entities for account deactivate.
- [ ] Create entities for account updating.
- [ ] Create entities for account fetching.

### Applications
- [ ] Create entities for profile fetching.

___

## UseCases

### Staff
- [ ] Create use-case register an application.
- [ ] Create use-case delete an application.
- [ ] Create use-case update an application name and description.

### Managers
- [ ] Create use-case create an role.
- [ ] Create use-case delete an role.
- [ ] Create use-case update a role name and description.
- [ ] Create use-case update a role permission (upgrade or downgrade).

### DefaultUsers
- [ ] Create use-case to register an account.
- [ ] Create use-case to delete an account.
- [ ] Create use-case to update an account.
- [ ] Create use-case to guest a user by email.
- [ ] Create use-case to remove a guest.

### Applications
- [ ] Create use-case to fetch profile from e-mail.

___

## Adapters - SQL

### Applications
- [ ] Create prisma adapter for applications registration.
- [ ] Create prisma adapter for applications deletion.
- [ ] Create prisma adapter for applications updating.
- [ ] Create prisma adapter for applications fetching.

### Users
- [ ] Create prisma adapter for users registration.
- [ ] Create prisma adapter for users deletion.
- [ ] Create prisma adapter for users updating.
- [ ] Create prisma adapter for users fetching.

### Roles
- [ ] Create prisma adapter for roles registration.
- [ ] Create prisma adapter for roles deletion.
- [ ] Create prisma adapter for roles updating.
- [ ] Create prisma adapter for roles fetching.

### Guests
- [ ] Create prisma adapter for guests registration.
- [ ] Create prisma adapter for guests deletion.
- [ ] Create prisma adapter for guests updating.
- [ ] Create prisma adapter for guests fetching.

### AccountTypes
- [ ] Create prisma adapter for account-types registration.
- [ ] Create prisma adapter for account-types deletion.
- [ ] Create prisma adapter for account-types updating.
- [ ] Create prisma adapter for account-types fetching.

### Accounts
- [ ] Create prisma adapter for accounts registration.
- [ ] Create prisma adapter for accounts deletion.
- [ ] Create prisma adapter for accounts updating.
- [ ] Create prisma adapter for accounts fetching.

___

## Ports - API

Not already defined.
