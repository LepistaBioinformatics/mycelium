# Changelog

All notable changes to this project will be documented in this file.

## [8.0.1-beta.3] - 2025-07-09

### 🚀 Features

- *(operations-discovery)* Implements endpoints to perform operations discovery

### 🚜 Refactor

- *(libs)* Move auxiliary libs to a dedicated directory

## [8.0.1-beta.2] - 2025-07-06

### 🚀 Features

- Upgrade connection strings to allow roles and account-ids instead of permissioned roles
- Implements an option co create role related accounts for spurious roles

### 🐛 Bug Fixes

- Avoid to expose system message to external customers
- Set spurious role related accounts as no std

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.1] - 2025-06-30

### 🚀 Features

- *(written-by)* Include the written by field into the webhooks model
- *(subscription-manager-account)* Create a use-case to initialize the subscription manager account with role scoped account type
- *(guest-to-subs-manager-account)* Create methods to guest and revoke to subscription management accounts
- *(tenant-wide-permissions)* Upgrade use-cases to use the tenant-wide permissions checker throug profile

### ⚙️ Miscellaneous Tasks

- *(connection-string)* Encode connection strings
- Release

## [8.0.0] - 2025-06-27

### 🚀 Features

- *(delete-account)* Reduce scope of account deletions and include users account soft deletion
- *(redact-email)* Expose the redact-email function to allow redact emails from simple strings
- *(get-owner-id)* Include the owner id to the connection strings and create a getter to collect it
- *(connection-strings)* Replace the multi-type connection strings by a single user-related connection string

### 🐛 Bug Fixes

- Include deletion flag through the account management in application
- *(tenant-owner)* Remoce the tenant owner checking as a role
- *(account-status-downgrade)* Include the possibility to new users downsgrade the own account without staff position
- *(tenant-owner)* Include check for tenant ownership in all tenant-manager actions
- *(email-redaction)* Replace all direct references to the user email in use-cases tracing by the redacted version
- *(remove-owner-id)* Remove the owner id information from the public part of the connection strings
- *(account-updating)* Fix the parsing of the field update-by during account updating operations

### 🚜 Refactor

- *(connection-string)* Review the applicaiton middlewares to allow users to access the full api interface with connection strings

### ⚙️ Miscellaneous Tasks

- Release

## [7.13.3] - 2025-06-22

### 🚀 Features

- *(proxy)* Implements a proxy feature to be applied at the service level

### 🐛 Bug Fixes

- *(soft-delete)* Implements the soft deletion of accounts

### ⚙️ Miscellaneous Tasks

- Release
- Release

## [7.13.1] - 2025-06-12

### ⚙️ Miscellaneous Tasks

- Release

## [7.13.0] - 2025-06-07

### 🚀 Features

- *(paginated-services)* Implements pagination to the services listing
- *(paginated-routes)* Include pagination in routes list endpoint
- *(tools-discoverability)* Do implements the downstream routes discoverability

### ⚙️ Miscellaneous Tasks

- Release

## [7.12.0] - 2025-05-12

### 🚀 Features

- *(allowed-sources)* Wip - move the definition of the allowed souces struct attribute from route to service
- *(allowed-sources)* Upgrade temporary structs that load services and routes do respond to the allowed-sources moving
- *(webhook events)* Register the webhook trigger for accounts update and delete

### 🐛 Bug Fixes

- *(webhook-triggers)* Reactivate the webhook triggers for put and delete actions

### 🚜 Refactor

- *(route-dto)* Rename the group struct attribute to security-group to better intent their usage

### ⚙️ Miscellaneous Tasks

- Release
- Release

## [7.10.0] - 2025-05-03

### 🚀 Features

- *(downstream-tracing)* Configure traces and attributes to track downstream routes metrics

### ⚙️ Miscellaneous Tasks

- Release

## [7.9.2] - 2025-04-27

### 🚀 Features

- *(health-check)* Replace the database based health check by a open-telemetry based health check system

### 💼 Other

- *(git-cliff)* Upgrade all workspace packages
- *(git-cliff)* Upgrade all workspace packages

### ⚙️ Miscellaneous Tasks

- Release
- Release
- Release

## [7.9.0] - 2025-04-23

### 🚀 Features

- *(git-cliff)* Introduce the git-cliff fot project changelog management
- *(git-cliff)* Track version upgrading into changelogs

### 💼 Other

- 7.8.1 → 7.8.2.dev1 [skip-ci]
- *(bump)* Manual fix the package version

### ⚙️ Miscellaneous Tasks

- Release

## [7.8.2] - 2025-04-22

### 💼 Other

- 7.8.1 → 7.8.2.dev1 [skip-ci]
- 7.8.2.dev1 → 8.0.0.dev2 [skip-ci]
- Upgrade version scheme to adequate to semver

### ⚙️ Miscellaneous Tasks

- Fix version of commitizen

## [7.8.1] - 2025-04-22

### 💼 Other

- 7.8.0 → 7.8.1 [skip-ci]

## [7.8.0] - 2025-04-22

### 🚀 Features

- *(tools)* Wip - implements changes to serve discoverable services through endpoint
- Wip - replace the lazy-static implementation of the memory database by a shaku injectable module
- Finish the migration of the lazy-static for in-memory storage of soutes to use shaku injection of in-memory data
- *(multihost-support)* Add support for multiple hosts for downstream services
- Wip - implements in memory tracking for health check metrics
- Finish the implementation of the healthcheck-logs registration in database
- Improve the tools discovery endpoint to include more information about context and capabilities

### 🐛 Bug Fixes

- *(tools)* Reduce information of the services available during discoverability
- Turn service id required field

### 💼 Other

- 7.7.13 → 7.8.0 [skip-ci]

## [7.7.13] - 2025-04-01

### 🐛 Bug Fixes

- Include a slug filtration during guest roles fetching

### 💼 Other

- 7.7.12 → 7.7.13 [skip-ci]

## [7.7.12] - 2025-03-30

### 🚀 Features

- Include the possibility to tenant-owners to request tenant details using the tenant-manager endpoint

### 🐛 Bug Fixes

- *(get-tenant-details)* Handle get-ids-or-error result dispatch to avoid young return with error on check if user is tenant-manager
- Use owner ids instead of the account id during filtration of the tenant details

### 💼 Other

- 7.7.9 → 7.7.10 [skip-ci]
- 7.7.10 → 7.7.11 [skip-ci]
- 7.7.11 → 7.7.12 [skip-ci]

## [7.7.9] - 2025-03-29

### 🐛 Bug Fixes

- *(profile)* Reconigure permissions to allow greater or equal instead to equal only on check user profile permissions

### 💼 Other

- 7.7.8 → 7.7.9 [skip-ci]

## [7.7.8] - 2025-03-29

### 🐛 Bug Fixes

- Remove the read-write option from permissions to avoid ambiguous filtering of profile options

### 💼 Other

- 7.7.7 → 7.7.8 [skip-ci]

## [7.7.7] - 2025-03-28

### 💼 Other

- 7.7.6 → 7.7.7 [skip-ci]

## [7.7.6] - 2025-03-28

### 💼 Other

- 7.7.5 → 7.7.6 [skip-ci]

## [7.7.5] - 2025-03-27

### 💼 Other

- 7.7.4 → 7.7.5 [skip-ci]

## [7.7.4] - 2025-03-27

### 💼 Other

- 7.7.3 → 7.7.4 [skip-ci]

## [7.7.3] - 2025-03-26

### 💼 Other

- 7.7.2 → 7.7.3 [skip-ci]

## [7.7.2] - 2025-03-26

### 💼 Other

- 7.7.1 → 7.7.2 [skip-ci]

## [7.7.1] - 2025-03-26

### 💼 Other

- 7.7.0 → 7.7.1 [skip-ci]

## [7.7.0] - 2025-03-26

### 🚀 Features

- Implements new created and updated fields and propagate this fields along the software stack
- Implements new created and updated fields and propagate this fields along the software stack
- Implements a new endpoint to serve account details to the account owners from the beginners api group
- Implements an endpoint to serve tenant information for tenant-manager accounts
- Do implements pagination when list guest users on account

### 🐛 Bug Fixes

- *(update-account-name-and-flags)* Include a logic to avoid the updating of the account slug after update account name in update-account-name-and-flags use-case
- *(create-default-account)* Upgrade the create-default-account use-case to generate the account slug from the user principal email
- *(downgrade-account-privileges)* Include a check to deny downgrade operations in non-self accounts
- *(create-management-account)* Set the is-detault flag before persist the new created account to the datastore
- Populate the created-by field during the creation of a connection between two guest-roles
- Populate the created-by field during the creation of a connection between two guest-roles

### 💼 Other

- 7.6.0 → 7.7.0 [skip-ci]

## [7.6.0] - 2025-03-14

### 🚀 Features

- Implements a system flag in guest-roles to indicate roles restricted to system accounts

### 💼 Other

- 7.5.11 → 7.6.0 [skip-ci]

## [7.5.11] - 2025-03-10

### 🐛 Bug Fixes

- Upgrade production configurations to load templates as a project artifact

### 💼 Other

- 7.5.10 → 7.5.11 [skip-ci]

## [7.5.10] - 2025-03-10

### 🚀 Features

- Remove the redis dependency from notifier system and replace by postgres dependency

### 💼 Other

- 7.5.9 → 7.5.10 [skip-ci]

## [7.5.9] - 2025-03-10

### 🚀 Features

- Implements a ping test to the email dispatcher initialization

### 💼 Other

- 7.5.8 → 7.5.9 [skip-ci]

## [7.5.8] - 2025-03-06

### 🐛 Bug Fixes

- Upgrade account list to allow non tenant requests and refine the account filtering based on the user roles
- Remove lock for subscription account on list guest to account
- Include the guest role id during the guest user connection to account

### 💼 Other

- 7.5.7 → 7.5.8 [skip-ci]

## [7.5.7] - 2025-03-04

### 🚀 Features

- Implements a guest-role fetching details for subscriptions managers

### 💼 Other

- 7.5.6 → 7.5.7 [skip-ci]

## [7.5.6] - 2025-03-04

### 🚀 Features

- Implements a subscriptions-manager endpoint group to list guest roles

### 💼 Other

- 7.5.5 → 7.5.6 [skip-ci]

## [7.5.5] - 2025-03-01

### 🐛 Bug Fixes

- Turn webhooks paginated

### 💼 Other

- 7.5.4 → 7.5.5 [skip-ci]

## [7.5.4] - 2025-02-26

### 🚀 Features

- Create a new option to allow authenticated users to interact with the mycelium downstream routes rithout registration
- Do implements the public tenant fetcher

### 💼 Other

- 7.5.3 → 7.5.4 [skip-ci]

## [7.5.3] - 2025-02-19

### 💼 Other

- 7.5.2 → 7.5.3 [skip-ci]

## [7.5.2] - 2025-02-19

### 🐛 Bug Fixes

- Increase verbosity of the user creation process

### 💼 Other

- 7.5.1 → 7.5.2 [skip-ci]

## [7.5.1] - 2025-02-18

### 🐛 Bug Fixes

- Include a desynchronozation element to avoid multiple synchronous execution of email and webhook dispatcher

### 💼 Other

- 7.5.0 → 7.5.1 [skip-ci]

## [7.5.0] - 2025-02-16

### 💼 Other

- 7.4.0 → 7.5.0 [skip-ci]

## [7.4.0] - 2025-02-10

### 💼 Other

- 7.3.0 → 7.4.0 [skip-ci]

## [7.3.0] - 2025-02-09

### 💼 Other

- 7.2.0 → 7.3.0 [skip-ci]

## [7.2.0] - 2025-02-05

### 🚀 Features

- Wip - implements the key profile persistence to the redis database
- *(cached-profile)* Finish the implementation for the profile caching

### 🐛 Bug Fixes

- Upgrade the credencials checker to dinamically load identity providers

### 💼 Other

- 7.1.0 → 7.2.0 [skip-ci]

### 🚜 Refactor

- Refactor project to inject notifier module instead instance along the api port

## [7.1.0] - 2025-01-31

### 🚀 Features

- Review the weghook trigger names to improve the user understanding of their goal
- Wip - do implements the asynchronous dispatching of webhooks
- Wip - implements the async dispatcher functionality

### 🐛 Bug Fixes

- Fix the webhook async dispatch to avoid updates of the payload package and mirror important changes to database
- Remove guesting and revoke options from the webhook trigger

### 💼 Other

- 7.0.0 → 7.1.0 [skip-ci]

### 🚜 Refactor

- Move tracing and async dispatchers to dedicated modules
- Comment not implemented triggers options for the webhook

## [7.0.0] - 2025-01-27

### 🚀 Features

- Implements the tenant ownership information into the profile
- Improve the profile owner filtration and apply the improvement to the tenant owner endpoints
- Implements the account metadata crud
- Implements the error code diesel repository and initialize modules for other entities
- Implements the guest_user diesel adapter
- Implements the tenant adapters for diesel

### 🐛 Bug Fixes

- *(use-cases)* Remove unused roles from profile filtering operation
- Include tenant at the profile filtering
- Include the url option to the tenants-ownership field of the profile dto
- Improve information about the account creation status on email checking response
- Fix the accout fetching adapter for prisma
- Fix the user token invalidation on create a new one
- Fix the totp lifecycle
- Fix the tenant fetching to migrato to native orm diesel query
- Fix the tenant fetching process
- Fix the tenant operations related to the diesel database engnie
- Fix the webhook updating options to avoid updation of the url and triggers
- Review the error code life cycle
- *(subscription-accounts)* Fix the subscription accounts related operations
- Review the guest roles related operations
- Fix the meta endpoints for account meta management

### 💼 Other

- 6.6.0 → 7.0.0 [skip-ci]

### 🚜 Refactor

- Rename the standard account flag on licensed resources to a system account
- Rename use-cases to better inform users about functionality
- *(profile)* Split profile elements into independent modules
- Allow dead code for the profile fetching using telegram user use-case
- Remove prisma client adapter

### 📚 Documentation

- Include the tracing level for the profile fetching from request cascade

### ⚡ Performance

- *(tenant)* Remove from serialization none fields of tenant dto
- *(account)* Remove from serialization none fields of account dto

## [6.6.0] - 2025-01-07

### 🚀 Features

- *(dto-profile)* Upgrade profile to include a filtration state after each filter operation
- Apply the new profile filtering validation
- Wip - review the guest system

### 🐛 Bug Fixes

- *(email-template)* Move the email subject to the templates file allowing the internationalization by file
- Inject the role name on guest to default account using service accounts
- Fix the permissioning system
- *(profile)* Inform about deprecation of direct filter methods of profile

### 💼 Other

- 6.5.0 → 6.6.0 [skip-ci]

## [6.5.0] - 2025-01-02

### 🐛 Bug Fixes

- Fix the env variable collectino and migrate all auth variables to dynamically collected ones

### 💼 Other

- 6.4.0 → 6.5.0 [skip-ci]

## [6.4.0] - 2025-01-02

### 🚀 Features

- Implements the secrets collection from vault

### 💼 Other

- 6.3.0 → 6.4.0 [skip-ci]

## [6.3.0] - 2024-12-31

### 🚀 Features

- Implements the invitation acceptance use cases and api
- Implements the gateway routes basic elements to check endpoints by api
- Implements the secrets service collection during the api gateway initialization
- Implements the injection of secrets through the gateway router
- Implements a new functionality to create all system roles by managers
- *(gateway-manager/service)* Implements the service listing for gateway managers
- *(user-account-creation)* Include a email notification to the new account creation workflow

### 🐛 Bug Fixes

- Fix the endpoints security definition
- Include additional checks to allow routing to insecure downstream paths only if explicitly informed by users
- Review the licensed resources filtering from database
- Remove unused test
- Fix the parsing and verification of connection strings not working
- Fix the webhook dispatching to decrypt secrets befhre send request to the incoming route

### 💼 Other

- 6.2.0 → 6.3.0 [skip-ci]

### 🚜 Refactor

- Rename the use-cases to mirror the application roles
- Move the match-forward-address to the api-port-router functionality
- *(secret-dto)* Move the webhook secret dto to a independent dto named http-secret
- Refactor the route match use-case to use a correct base response from mycelium
- Rename the env-or-value functionality by secret-resolver
- Move the match forward address use-case to the gateway use-cases group
- Remove the role submodule and move chindren modules to the root parent

### 📚 Documentation

- *(use-cases)* Brief documenting the use-cases mod file

## [6.2.0] - 2024-12-01

### 🚀 Features

- Review the full api documentation and endpoints locations to improve the development experience and usability

### 💼 Other

- 6.1.0 → 6.2.0 [skip-ci]

## [6.1.0] - 2024-11-24

### 🚀 Features

- Upgrade the profile management to inject licensed resources as a url instead of a json object
- Implements the fetching the connection string from the request header

### 🐛 Bug Fixes

- Fix the service endpoints to collect the tenant id from the connection string itself

### 💼 Other

- 6.0.0 → 6.1.0 [skip-ci]

### 🚜 Refactor

- Centralize the platform name and the platform url as the domain config instead to inject from the request url
- Implements basic locale to the email template
- Replace the tenant id from the api route to use a x-mycelium-tenant-id header
- Refactor all routes to be more consistent
- Refactor the no-role guest endpoint to the new service route group

### ⚡ Performance

- Improve the profile injection on internal roles to filter roles before send by downstream requests

## [6.0.0] - 2024-11-13

### 🚀 Features

- Upgrade the internal authentication flow to generate simple authentication tokens from mycelium
- Wip - initialize the migration from activation url to numeric token on create new user accounts
- Improve the user activation code
- Implements the base for opentelemetry in lycelium
- Replace logs from the core use-cases by tracing
- Implements the password recovery flow
- Wip - do implement the base logic fot the tenant implementation
- Wip - do implement the new tenant based accounts management
- Wip - do implement the tenant-owner functionalitie
- Do implement the tenant owner guest and revoke use-cases
- Implement the account deletion repo
- Implement use-cases for metadata management in tenant-owner modules
- Implements the abstract code for tenant tag management
- Wip - implements the prisma repository for tenant registration
- Do implements the tenant management for prisma repositories
- Re-include the myc-cli crate on the project after fixing
- Implements the tenant management endpoints
- Implements the tenant-manager endpoint related elements
- Wip - implements the guest role children insertion and deletion features
- Implements the route level filtration by role
- Implements the route filtration by permissioned roles
- Implements the guest-to-children-account use-case as a api port endpoint
- Implements the connection string elements to generate service tokens
- Implements the prisma adapter to create new connection string tokens and remove unused imports from native-errors in endpoints
- Implements the noreply and internal emails naming
- Implements the token creation endpoint of guest-manager
- Implements the prisma and api injectors for token fetching module
- Implements the totp encryption and decryption
- Implements the secret encryption during the totl registration on user mfa flow
- Implements the totp initial steps for the otp registration
- Implements the totp activation
- Implements the two setp login using totp flow
- Implements the totp disable
- Upgrade the azure authorization flow in replacement to the remote check

### 🐛 Bug Fixes

- Move the email template location and fix the email verification code generation
- Fix return tyoe of login function
- Solve the permission check for the tenant operations to include validation over the manager and staff accounts
- Fix role-related account propagation through the webhook element
- Rename revoke tenant owner wrongly named use-case function
- Migrate sql database before fix the prisma repositories
- Fix database connection between tenant and account
- Fix account and licensed resources prisma repositories
- Fix the email consumption queue processor
- Replace the guesting email template element to use the tera template
- Fix the guest process
- Fix the email processing counter and fix the child role invitation use-case to avoid guest to different roles that the target one
- Replace the myc00013 response from profile permissions check by a myc00019 code
- Include the redaction function on get webhook from database
- Fix the webhook lifecycle to live as a more verbose to the final users
- Adjust the webhook dispatching method based on the webhook trigger
- Fix the guest to default subscription account name inserted at the account name
- Remove url query string from webhook response to avoid expose secrets
- Review the account list method to allow filter directly by account-type

### 💼 Other

- 5.0.8 → 6.0.0 [skip-ci]

### 🚜 Refactor

- Move the session_token to account-life-cycle module
- Rename the user and subscription manager roles
- Move the staff tenant management elements to manager management elements
- Move the guest to subscription-manager instead to guest-managers
- Adequate all core elements to include tenant on the profile filtering
- Rename the checked status to verified or unverified in verbose-status
- Refactor the mycelium smtp to be a general purpose notifier
- Move the guest-role to a dedicated dto module
- Rename child insertion and removal use-cases files
- Replace all crud related permissioner elements by simple read-write ones
- [**breaking**] Refactor the permissions to be a integer with read write and read-write options only
- Rename user to users and subscription by subscriptions as default actors and mirror to dependent elements
- Move the email sender to a dedicated module shared between use-cases and create a new mapped-error-to-http-response mapping handled
- Rename the token generator for the account associated connection string use-case
- Refactor webhooks to follow de main stream format widely used in web applications
- Move the shared functions from the roles module to a support module

### 📚 Documentation

- Include note about the roles that should perform actions on tenant-owner -- tenant use-cases
- Refactor the account-scopped-connection-string-meta module doc header

### 🧪 Testing

- Include a basic test for account-type

## [5.0.8] - 2024-04-25

### 💼 Other

- 5.0.7 → 5.0.8 [skip-ci]

## [5.0.7] - 2024-04-12

### 💼 Other

- 5.0.6 → 5.0.7 [skip-ci]

## [5.0.6] - 2024-04-10

### 💼 Other

- 5.0.5 → 5.0.6 [skip-ci]

## [5.0.5] - 2024-04-10

### 💼 Other

- 5.0.4 → 5.0.5 [skip-ci]

### 🧪 Testing

- Fix test broken on the previous project tag

## [5.0.4] - 2024-04-09

### 💼 Other

- 5.0.3 → 5.0.4 [skip-ci]

## [5.0.3] - 2024-04-08

### 🐛 Bug Fixes

- *(account-and-users)* Turn the default user and account creation process to considering email as case-insensitive

### 💼 Other

- 5.0.2 → 5.0.3 [skip-ci]

## [5.0.2] - 2024-03-21

### 💼 Other

- 5.0.1 → 5.0.2 [skip-ci]

## [5.0.1] - 2024-03-11

### 💼 Other

- 5.0.0 → 5.0.1 [skip-ci]

## [5.0.0] - 2024-03-09

### 💼 Other

- 4.16.0 → 5.0.0 [skip-ci]

### ⚡ Performance

- *(licensed-resources)* [**breaking**] Replace the licensed resources fetching to use a view instead to perform multiple joins to fetch licenses contents

## [4.16.0] - 2024-02-26

### 🚀 Features

- Replace error factories to accept generic types as the error message argument

### 💼 Other

- 4.15.3 → 4.16.0 [skip-ci]

## [4.15.3] - 2024-02-22

### 💼 Other

- 4.15.2 → 4.15.3 [skip-ci]

## [4.15.2] - 2024-02-21

### 💼 Other

- 4.15.1 → 4.15.2 [skip-ci]

## [4.15.1] - 2024-02-15

### 🚀 Features

- *(subscription-account)* Implements a use-case and endpoint to update accounts name and flags
- Implements the new base package to replace the clean-base package
- Upgrade webhook propagation functionalities to passthrough the bearer token together request
- Implements the slug name to allow accounts renaming without rename
- Implements the tag models into domain dtos and adapters
- Implements the tags creation endpoint entities and use-cases
- Implements a reelated account enumerator that allow users to check permissions for a specified account or itself has privileged permissions

### 🐛 Bug Fixes

- Implementing serialization and deserialization into the permissions action type enumerator
- Fix action type
- *(router)* Fix http method filtragion from gateway router
- *(router)* Fix router to correctely check for all and none methods
- Upgrade all std use-cases to responds to default account on check actinos permissions from profile
- *(subscription-account-propagation)* Fix the subscription account propagation use-cases
- Fix the subscription accounts search to include tag search and a case insensitive search including uuid targeted search
- Implements related accounts filtering default methods
- Orient all internal paths dependencies to the project path instead to use directly into the workspace

### 💼 Other

- 4.15.0 → 4.15.1 [skip-ci]

### 🚜 Refactor

- Refactor the cargo dependencies to import shared dependencies from the workspace

### ⚙️ Miscellaneous Tasks

- Replace all workspace reference to a single line notation

## [4.7.5] - 2023-12-17

### 🐛 Bug Fixes

- Fix the response from account creation to return a 409 code if account exists

### 💼 Other

- 4.7.4 → 4.7.5 [skip-ci]

## [4.7.4] - 2023-12-17

### 💼 Other

- 4.7.3 → 4.7.4 [skip-ci]

## [4.7.3] - 2023-12-17

### 🐛 Bug Fixes

- Move the email notifier on generate tokens to be collected form environment

### 💼 Other

- 4.7.2 → 4.7.3 [skip-ci]

## [4.7.2] - 2023-12-17

### 🐛 Bug Fixes

- *(default-account-creation)* Include identity extraction from request token on create default accounts

### 💼 Other

- 4.7.1 → 4.7.2 [skip-ci]

## [4.7.1] - 2023-12-17

### 💼 Other

- 4.7.0 → 4.7.1 [skip-ci]

## [4.7.0] - 2023-12-14

### 🚀 Features

- Implements webhook updating and listing

### 💼 Other

- 4.6.1 → 4.7.0 [skip-ci]

## [4.6.1] - 2023-12-07

### 💼 Other

- 4.6.0 → 4.6.1 [skip-ci]

### 🚜 Refactor

- Rename accessor method of env-or-value from get to get-or-error to indicate that the mathod returns a result

## [4.6.0] - 2023-12-06

### 🚀 Features

- Implements the collection of secret values from environment instead of to use hardcoded configurations

### 💼 Other

- 4.5.1 → 4.6.0 [skip-ci]

## [4.5.1] - 2023-12-04

### 💼 Other

- 4.5.0 → 4.5.1 [skip-ci]

## [4.5.0] - 2023-12-04

### 🚀 Features

- Implements the auxiliary endpoints

### 💼 Other

- 4.4.0 → 4.5.0 [skip-ci]

## [4.4.0] - 2023-12-04

### 🚀 Features

- Refactores standard and managers endpoints to mirror the new actors system

### 💼 Other

- 4.3.0 → 4.4.0 [skip-ci]

### 🚜 Refactor

- Turn default-user endpoints and apis to mirrir the new default system actors
- Move manager use-cases to dedicated actors folders
- Move error codes to system-manager role use-cases group

## [4.3.0] - 2023-12-03

### 🚀 Features

- Implements new notifications and improve the accounts creation flow

### 💼 Other

- 4.2.0 → 4.3.0 [skip-ci]

## [4.2.0] - 2023-10-25

### 🚀 Features

- Upgrade prisma adapter user model to include providers as options
- Implements the email check function to start the proccess of authentication
- Finish implementation of the user and account registrations in two independent steps
- Create is-principal field to indicate users which created the accounts
- Turn accounts creation process to possible without user registration
- Wip - start implementation of the session token management during users accounts lyfe cycle
- Wip - implements the config manager module
- Migrate session-token management from redis to postgres
- Implements configuration passthrough from api port to another application layers

### 🐛 Bug Fixes

- Include error handling if some exception occurred during the user registration
- Upgrade staff account creation flow to include password and hash into data persistence modules
- Fix the user fetching and registration adapters to include and omit password informations and init creation of users endpoints
- Remove unused commitizen from redis cardo toml

### 📚 Documentation

- Documenting the check-email-registration-status use-cas

## [4.1.1] - 2023-09-19

### 🚀 Features

- Upgrade router to allow http2 service as downstream url for apis management
- Wip - start creation of the default account management
- Wip - start creation of the token management elements
- Create base attributes to implement password checks to users
- Move new method of user new_with_provider method allow the provider field to be required during default object creation

### 🐛 Bug Fixes

- Fix account and user prisma adapters to deals with user provider models
- Finish use-cases related to session token registration and expiration
- Remove non domain logic from the session token domain dto
- Upgrade account creation use-cases to include or not profile information during accounts initializations
- Upgrade user model to remove password hash and salt information before serialize user object
- Turn provider and their password hash and salt as private fields
- Fix prisma adapter to deals with the profile field as private

### 💼 Other

- Synchronize package sub versions
- Upgrade from 3 to 4 the major package version
- Partial increment package versions
- 4.1.0 → 4.1.1 [skip-ci]

### 🚜 Refactor

- Rename account propagation response during webhook actions
- Rename config loading use-case to indicate that config is loaded from yaml file not json
- Move all entities to dedicated folders mirriring their role in the application

### 🧪 Testing

- Fix test implementation error whitch try to transit between pending to inactive accounts

## [4.0.0] - 2023-09-07

### 🚀 Features

- Wip - implements entities for webhook management
- Wip - upgrade management use-cases to dealing with webhooks during subscription accounts creation
- Create webhook base model
- Wip - upgrade ports to work with with webhooks
- Implements the webhooks creation and deletion usecases adapters and ports
- [**breaking**] Upgrade the account model to include multiple owners allowing to work with multi-user accounts with the same real world identity

### 🐛 Bug Fixes

- Extend previous commit

## [3.0.1] - 2023-07-29

### 💼 Other

- 3.0.0 → 3.0.1 [skip-ci]

### ⚡ Performance

- Move user creation of the account creation process to a transaction into the account creation

## [3.0.0] - 2023-06-18

### 🚀 Features

- [**breaking**] Remove the include-itself parameter of the profile functions that check privilegies

### 💼 Other

- 2.0.0 → 3.0.0 [skip-ci]

## [2.0.0] - 2023-06-18

### 💼 Other

- 1.0.0 → 2.0.0 [skip-ci]

### ⚙️ Miscellaneous Tasks

- Manual upgrade all versions of the mycelium package

## [1.0.0] - 2023-06-18

### 🚀 Features

- Include methods to filter ids of accounts that profile has permissions
- Update profile methods to get ids by permission to include a list of roles instead of a single role
- Replace the id type of the get method by uuid
- Replace the guest-role deletion return type from dto to uuid
- Implements the use-case and entities and their dtos to perform token registration and deregistration
- Implements the token validation data repositoryes and dependencies
- Update registration and de-registration adapter to group tokens by date allowing cleanup every day
- Implements the token cleanup entities and redis adapters and prepare injection modules into api ports
- Limit the visivility of the token-expiration-time config to crate
- Update the profile fetch use-case to include the validation token creation in the process
- Update all package dependencies to use unsufixed dtos and update fetch profile use-case to include new arguments
- Update the profile fetch api response
- Include profile response elements as the exportation elements of the api-public port
- Update the cli port that creates the seed staff account
- Implement account endpoints of the managers api group
- Refacror the manager endpoints to split routes by group
- Imlement endpoints for token management by service users
- Update seed staff creation to include is-manager flag at the created account
- Implement endpoints from shared use-cases group to manager endpoints group
- Creeate staff endpoints for accounts management
- Update profile extractor to validate token
- Update descriptions of default accounts
- Implement endpoints to get and list subscription accounts
- Implements entpoint to fetch guest-users using the subscription account id
- Implements list methods for role and guest-roles
- Create a new licensed-resources adapter to replace the guest-roles adapter used during profiles fetching
- Include an optional parameter to allow exclude the own id from from the licensed profiles list
- Split fetch profile pack use-case in two sepparate use-cases and create a new default-users endpoint to provide profiles
- Start creation of the api gateway
- Create api gateways functionalities to turn mycelium independent
- Update extractor and credentials checker to work around profiles instead profile-packs
- Create methods to fetch user information from ms graph using me route
- Implements the second step for identity checking on fetch profile from the api request
- Implements all features to list accounts by type and include archivement options to database
- Implements the endpoint to get all guests of a default account
- Implements endpoint to change account archival status
- Convert the account type from the managers account list to a flag indicating if only subscription account should be fetched
- Implements the verbose status notation to turn the status interpretation from tags to text
- Replace the accounts filtration from using flags to use accounts verbose statuses
- Implements the uninvite system for guests
- Implements the unapproval use-cases and appropriate endpoints
- Include the possibility to jump from active users to archived users in managers use-cases
- Include the account name and guest-role name into the licensed resources object
- Implement methods to update the guest-role of an existing guest
- Replace all manual error handling on use mapped-errors to infalible implementation
- Upgrade all gateway use-cases and some remaining use-cases from roles group to use infallibles
- Create the error code dto to manage code data
- Upgrade version of the clean-base error handlers at mycelium
- Implements entities for error code crud management
- Implements the error code prisma crud and include message code to prisma client
- Implement use-cases to manage error codes
- Implements the api endpoints for error-codes management
- Implements the batch creation of the native error persistence
- Create methods to check ids permissinos with error status if no licensed ids were found
- Include user names into the profile object
- Update user related dtos to include partial equals as derive and upgrade get ids elements of profile
- Turn default include-itself parameter of profile as false
- [**breaking**] Wip - implements the app interface for management and create the commitizen file for auto versioning

### 🐛 Bug Fixes

- Fix the privilegies misverification on the account activation use-case
- Update visibility of the default account creation use-case to be accessible only from crates
- Fix guest-role use-case to include role id as optional filtration argument
- Fix regex validation for email string parsing and include new tests for it
- Include a string parser on the permissions-type enumerator to allow parse permissions from api request
- Fix bug into the collection of profile during collection of the guest accounts
- Include a debug log after build and collect profile
- Upgrade version of clean-base and utoipa to fix downgrade dependencies
- Fix the profile validation and remove all code from token checking
- Fix bug during the creation of the guest users
- Fix bug repository to fetch licensed ids
- Fix bug during the approval or activation status change of an account
- Upgrade the status state checker to allow users to migrate to archived from inactive users
- Set checked as true during the creation of subscription accounts
- Resolve the error on uninvite guest to account
- Create a code field to present the concatenated prefix and error_number as the code

### 💼 Other

- Include the serde version on chrono to avoid incompatibility between the serde version used by mycelium and the chrono package
- Change the package name from mycelium to myc to avoid incompatibility with another existing crate package
- [**breaking**] Upgrade the package version
- Update categories of cargo file of myc-core
- Upgrade the crate version
- Update the myc-core crate version
- Upgrade myc-core version
- Upgrade myc-core version
- [**breaking**] Upgrade the myc-core crate version
- [**breaking**] Upgrade myc-core version
- Upgrade the myc-core package version
- Update all package versions
- Upgrade overall package versions before tag

### 🚜 Refactor

- Wip - move infra and smtp adapters to the adapters feature
- Move all antities from core packate to the root entities folder instead of seggregate by role
- [**breaking**] Re-export all entities from their parend module instead of publish their directly
- Refactore use-cases to re-export main case functions into the parent modules directelly
- Remove the dto notation from the domain dtos
- Move the upgrade and downgrade actions from shared to staff use-cases group
- Move account approval and status changes to the manager use-cases group
- Replace the default error imports from clean-base in use-case module to the new error factory import

### 📚 Documentation

- Includes package metadata in all subpackages of this project
- Fix the path to the readme file into the mycelium core package
- Update packages category slugs to match valid crate categories
- Update readmes of the main project and the core package
- Annotate profile-pack as deprecated
- Upgrade the status changes diagram
- Update the profile documentation

<!-- generated by git-cliff -->
