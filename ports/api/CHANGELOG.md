# Changelog

All notable changes to this project will be documented in this file.

## [8.1.1-rc.1] - 2025-11-11

### 🚀 Features

- Implements the profile compression on send if by http header

### 💼 Other

- Upgrade changelogs to mirror the beta.11 version

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.11] - 2025-10-27

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.10] - 2025-09-22

### ⚙️ Miscellaneous Tasks

- Release
- Release

## [8.1.1-beta.8] - 2025-09-22

### 🐛 Bug Fixes

- Remove header-based filtration of roles related to the account-manager endpoints

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.7] - 2025-09-17

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.6] - 2025-09-17

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.5] - 2025-09-17

### 🚀 Features

- Simplify the guesting process to share accounts with child roles

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.4] - 2025-09-16

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.3] - 2025-09-16

### 🐛 Bug Fixes

- Replace the subscriptions manager by account-manager scoped use-cases on list guest-roles

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.2] - 2025-09-16

### 🚀 Features

- Create dedicated endpoints to account-maangers to list guest-users

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.1-beta.1] - 2025-09-16

### ⚙️ Miscellaneous Tasks

- Release

## [8.1.0] - 2025-09-13

### 🐛 Bug Fixes

- Remove the mcp scope from the application

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.33] - 2025-09-03

### 🚀 Features

- Implements the single step account creation flow for verified accounts

### 🚜 Refactor

- Renamt the user account creation use-case to mirror their goal

### ⚙️ Miscellaneous Tasks

- Release
- Release
- Release

## [8.0.1-beta.30] - 2025-08-26

### 📚 Documentation

- Fix operation id of all endpoints using openapi operation-id key

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.29] - 2025-08-25

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.28] - 2025-08-25

### 🚀 Features

- Include the account name at the role-related account creation

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.27] - 2025-08-25

### 🐛 Bug Fixes

- Remove redundancy in accounts creation by subscription account managers

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.26] - 2025-08-21

### 🐛 Bug Fixes

- Fix mcp dependencies broken from 8.0.1-beta.25 version

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.25] - 2025-08-21

### 🚀 Features

- Wip - try to solve authentication into the mcp call tool

### 🐛 Bug Fixes

- Replace the properties parameter by parameter word on define mcp tool

### ⚙️ Miscellaneous Tasks

- Release
- Release

## [8.0.1-beta.23] - 2025-08-17

### 🚀 Features

- Insert comprehensive identifiers to the mycelium tokens
- Include a restriction tag to filter mcp tools before routing it to mcp server

### 🐛 Bug Fixes

- Fix authorization endpoints to allow oidc discovery with self registration

### ⚙️ Miscellaneous Tasks

- Release
- Release

## [8.0.1-beta.21] - 2025-08-11

### 🚀 Features

- Include the security group in downstream hequest header

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.20] - 2025-08-10

### 🧪 Testing

- Fix all non passing tests

### ⚙️ Miscellaneous Tasks

- Release
- Release

## [8.0.1-beta.18] - 2025-07-31

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.17] - 2025-07-30

### 🐛 Bug Fixes

- Return to the previous state on filter permissions to be greater or equal to profile perms

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.16] - 2025-07-30

### 🐛 Bug Fixes

- Rollback the permission check on licensed-resources fetching

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.15] - 2025-07-30

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.14] - 2025-07-30

### 🐛 Bug Fixes

- *(roles filtering)* Review the profile injection cascade to empower the connection string filtration priority

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.13] - 2025-07-29

### 🐛 Bug Fixes

- *(profile-filtration)* Solve the profile filtration bug that completely remove the roles during connnection string usage

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.12] - 2025-07-29

### 🐛 Bug Fixes

- Fix tools listing on unwrap summary

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.11] - 2025-07-29

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.10] - 2025-07-29

### 🚀 Features

- *(role-assiciated-accounts)* Do implement features that allow the role-associated accounts to be managed

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.9] - 2025-07-28

### 🚀 Features

- Implements the basis for servces discovery and mcp integration
- Wip - implements the basis for the mcp server management and connection
- Resover mcp return for list tools to include request body
- *(mcp-server)* Implements the basis for the mcp server execution
- *(mcp)* Finish the mcp server implementation

### 🐛 Bug Fixes

- *(operation-id)* Update operation-id building to include method service name

### 🚜 Refactor

- *(main-api-config)* Review the api configuration to group close elements given their system importance

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.8] - 2025-07-14

### 🐛 Bug Fixes

- Create a new resolution step to try to solve references in a recursive mode
- Update the operation-id to include a double underscore between service name and operation id to avoid conflicts

### 💼 Other

- Move the api dependencies from mycelium sibling packages to the workspace
- Move all cross dependencies of the project to the workspace definition

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.7] - 2025-07-11

### 🚜 Refactor

- Remove unused functions from the fetch-profile-from-request-token middleware

### ⚡ Performance

- Review the components resolution to avoid overresolution of components not rendered to the final users

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.6] - 2025-07-10

### 🐛 Bug Fixes

- *(remove unused packages from actix-web and fix licensed resources fetching sql syntax)* N

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.5] - 2025-07-09

### 🚀 Features

- *(security-groups)* Reduce the security group options to include only up to the protected-by-role option

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.4] - 2025-07-09

### 🚀 Features

- *(operations-discovery)* Implements endpoints to perform operations discovery

### 🚜 Refactor

- *(libs)* Move auxiliary libs to a dedicated directory

### ⚙️ Miscellaneous Tasks

- Remove undesired println from the router module
- Release
- Release

## [8.0.1-beta.2] - 2025-07-06

### 🚀 Features

- Implements an option co create role related accounts for spurious roles

### ⚙️ Miscellaneous Tasks

- Release

## [8.0.1-beta.1] - 2025-06-30

### 🚀 Features

- *(written-by)* Include the written by field into the webhooks model
- *(tenant-wide-permissions)* Upgrade use-cases to use the tenant-wide permissions checker throug profile

### ⚙️ Miscellaneous Tasks

- *(connection-string)* Encode connection strings
- Release

## [8.0.0] - 2025-06-27

### 🚀 Features

- *(delete-account)* Reduce scope of account deletions and include users account soft deletion
- *(connection-strings)* Replace the multi-type connection strings by a single user-related connection string

### 🐛 Bug Fixes

- Include deletion flag through the account management in application
- *(tenant-owner)* Remoce the tenant owner checking as a role
- *(tenant-owner)* Include check for tenant ownership in all tenant-manager actions

### 🚜 Refactor

- *(connection-string)* Review the applicaiton middlewares to allow users to access the full api interface with connection strings

### 🧪 Testing

- *(test-service)* Update the test service to allow run integration tests at the gateway level

### ⚙️ Miscellaneous Tasks

- Release

## [7.13.3] - 2025-06-22

### 🚀 Features

- *(proxy)* Implements a proxy feature to be applied at the service level

### 🐛 Bug Fixes

- Remove the status endpoint from the user group

### 📚 Documentation

- Fix the account hierarchy drawio chart to solve the cardinality of the account connnections

### ⚙️ Miscellaneous Tasks

- Release
- Remove function level deprecation message from status endpoint
- Remove the query parameter definition from status endpoint
- Release

## [7.13.1] - 2025-06-12

### ⚡ Performance

- Improve the score calculation performance on filter params

### 🧪 Testing

- Upgrade development elements to allows to test with real openapi documentation

### ⚙️ Miscellaneous Tasks

- Release

## [7.13.0] - 2025-06-07

### 🚀 Features

- *(paginated-services)* Implements pagination to the services listing
- *(paginated-routes)* Include pagination in routes list endpoint
- *(tools-discoverability)* Do implements the downstream routes discoverability
- Increment methods to fetch schemas defined at the route response

### 🐛 Bug Fixes

- Include independent filtration scores to filter tue query on graphql query root

### ⚙️ Miscellaneous Tasks

- Release

## [7.12.0] - 2025-05-12

### 🚀 Features

- *(webhook events)* Register the webhook trigger for accounts update and delete

### ⚙️ Miscellaneous Tasks

- Release

## [7.11.0] - 2025-05-10

### 🚀 Features

- *(allowed-sources)* Wip - move the definition of the allowed souces struct attribute from route to service
- *(gateway-router)* Wip - split the api router into steps to increase the legibility

### 🚜 Refactor

- *(route-dto)* Rename the group struct attribute to security-group to better intent their usage
- *(downstream secrets)* Move the secret injection to a dedicated module
- Move the gateway response builder to a dedicated module

### ⚙️ Miscellaneous Tasks

- Release
- Release
- Release

## [7.10.0] - 2025-05-03

### 🚀 Features

- *(otel)* Redirect logs to the otel collector instead to use the direct jaeger path
- *(downstream-tracing)* Configure traces and attributes to track downstream routes metrics
- *(otel)* Finish the implementation of the tracing injection by collector instead of the direct jaeger injection

### 🐛 Bug Fixes

- *(health-check-otel)* Include the hc00007 code into the final of the host health check operation
- Standardize the endpoint to inhect metrics into collector

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
- Wip - implements in memory tracking for health check metrics
- Finish the implementation of the healthcheck-logs registration in database
- Improve the tools discovery endpoint to include more information about context and capabilities

### 🐛 Bug Fixes

- *(tools)* Reduce information of the services available during discoverability
- Remove old module folder of the api-port submodule

### 💼 Other

- 7.7.13 → 7.8.0 [skip-ci]

## [7.7.13] - 2025-04-01

### 🐛 Bug Fixes

- Include a slug filtration during guest roles fetching

### 💼 Other

- 7.7.12 → 7.7.13 [skip-ci]

## [7.7.12] - 2025-03-30

### 💼 Other

- 7.7.9 → 7.7.10 [skip-ci]
- 7.7.10 → 7.7.11 [skip-ci]
- 7.7.11 → 7.7.12 [skip-ci]

## [7.7.9] - 2025-03-29

### 💼 Other

- 7.7.8 → 7.7.9 [skip-ci]

## [7.7.8] - 2025-03-29

### 🐛 Bug Fixes

- Remove the read-write option from permissions to avoid ambiguous filtering of profile options

### 💼 Other

- 7.7.7 → 7.7.8 [skip-ci]

## [7.7.7] - 2025-03-28

### 🐛 Bug Fixes

- Include an additional step during the tenants management account creation to register the manager account on tenant

### 💼 Other

- 7.7.6 → 7.7.7 [skip-ci]

## [7.7.6] - 2025-03-28

### 🐛 Bug Fixes

- Freeze zip version on try to avoid error on utoipa-swagger-ui

### 💼 Other

- 7.7.5 → 7.7.6 [skip-ci]

## [7.7.5] - 2025-03-27

### 🐛 Bug Fixes

- Downgrade utoipa to avoid nine version error on build

### 💼 Other

- 7.7.4 → 7.7.5 [skip-ci]

## [7.7.4] - 2025-03-27

### 🐛 Bug Fixes

- Upgrade project dependencies including dependabot issues

### 💼 Other

- 7.7.3 → 7.7.4 [skip-ci]

## [7.7.3] - 2025-03-26

### 🐛 Bug Fixes

- *(dependabot)* Introduce security fixes recommended by bependabot

### 💼 Other

- 7.7.2 → 7.7.3 [skip-ci]

## [7.7.2] - 2025-03-26

### 🐛 Bug Fixes

- *(utoipa-swagger-ui)* Upgrade utoipa-swagger-ui version to 9

### 💼 Other

- 7.7.1 → 7.7.2 [skip-ci]

## [7.7.1] - 2025-03-26

### 🐛 Bug Fixes

- *(utoipa-swagger-ui)* Include the reqwest feature flag in utoipa-swagger-ui to avoid to use curl during the swagger installation

### 💼 Other

- 7.7.0 → 7.7.1 [skip-ci]

## [7.7.0] - 2025-03-26

### 🚀 Features

- Implements a new endpoint to serve account details to the account owners from the beginners api group
- Implements an endpoint to serve tenant information for tenant-manager accounts
- Do implements pagination when list guest users on account

### 💼 Other

- 7.6.0 → 7.7.0 [skip-ci]

## [7.6.0] - 2025-03-14

### 🚀 Features

- Implements a system flag in guest-roles to indicate roles restricted to system accounts

### 💼 Other

- 7.5.11 → 7.6.0 [skip-ci]

### 📚 Documentation

- Include the myc logo to the docs assets

## [7.5.11] - 2025-03-10

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

### 💼 Other

- 7.5.7 → 7.5.8 [skip-ci]

### 📚 Documentation

- Update fetch guest role details endpoint swagger documentation

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

### 🚜 Refactor

- Replace the head endpoint to check the user status by a get method with body response

## [7.5.2] - 2025-02-19

### 💼 Other

- 7.5.1 → 7.5.2 [skip-ci]

## [7.5.1] - 2025-02-18

### 🐛 Bug Fixes

- Include a desynchronozation element to avoid multiple synchronous execution of email and webhook dispatcher

### 💼 Other

- 7.5.0 → 7.5.1 [skip-ci]

## [7.5.0] - 2025-02-16

### 🚀 Features

- Increase the ttl granularity of the cache for email and profile and the jwks response

### 🐛 Bug Fixes

- Update database model to be more migrationable

### 💼 Other

- 7.4.0 → 7.5.0 [skip-ci]

### 🚜 Refactor

- Convert the response status to a ok status on verity the email registration status endpoint

## [7.4.0] - 2025-02-10

### 🐛 Bug Fixes

- Replace the cached crate by a native implementation of the caching functions

### 💼 Other

- 7.3.0 → 7.4.0 [skip-ci]

## [7.3.0] - 2025-02-09

### 🚀 Features

- Include an extractor to check already the userinfo from the audience list

### 🐛 Bug Fixes

- Fix the email discovery process to include the user info collection from remote server

### 💼 Other

- 7.2.0 → 7.3.0 [skip-ci]

## [7.2.0] - 2025-02-05

### 🚀 Features

- Implements the userinfo cache
- Refactor the mycelium notifier to move the redis config init to a shared module
- Wip - implements the key profile persistence to the redis database
- *(cached-profile)* Finish the implementation for the profile caching

### 🐛 Bug Fixes

- Upgrade the credencials checker to dinamically load identity providers
- Re-introduce the internal provider to the issuer fetcher flow

### 💼 Other

- 7.1.0 → 7.2.0 [skip-ci]

### 🚜 Refactor

- Fix english words
- *(fetch_profile_from_request)* Split the fetch_profile_from_request to multiple submodules to turn the module arch as screamming
- Refactor email fetcher middleware to turn it more verbose and dev friendly
- Refactor project to inject notifier module instead instance along the api port
- Split notifier models to a dedicated submodules and initialize the kv lib

## [7.1.0] - 2025-01-31

### 🚀 Features

- Wip - do implements the asynchronous dispatching of webhooks
- Wip - implements the async dispatcher functionality

### 🐛 Bug Fixes

- Fix the webhook async dispatch to avoid updates of the payload package and mirror important changes to database

### 💼 Other

- 7.0.0 → 7.1.0 [skip-ci]

### 🚜 Refactor

- Move tracing and async dispatchers to dedicated modules

## [7.0.0] - 2025-01-27

### 🚀 Features

- Implements the tenant ownership information into the profile
- Improve the profile owner filtration and apply the improvement to the tenant owner endpoints
- Implements the account metadata crud
- Migrate all prisma dependencies to diesel

### 🐛 Bug Fixes

- Include tenant at the profile filtering
- Include the url option to the tenants-ownership field of the profile dto
- Improve information about the account creation status on email checking response
- Include the tenant-fetching repo to the tenant endpoints for tenant-owners
- Migrate the raw sql implementations injection of the fetch-profile-from-request to a native shaku module injection
- Replace diesel uuid in models and repositories by string
- Fix the profile fetching diesel query
- Fix the user token invalidation on create a new one
- Fix the totp lifecycle
- Fix the tenant fetching to migrato to native orm diesel query
- Fix the tenant fetching process
- Fix the tenant operations related to the diesel database engnie
- Fix the webhook updating options to avoid updation of the url and triggers
- Wip - fix the guest roles diesel orm functionalities
- *(subscription-accounts)* Fix the subscription accounts related operations
- Review the guest roles related operations
- Fix the guest to children account
- Fix the meta endpoints for account meta management

### 💼 Other

- 6.6.0 → 7.0.0 [skip-ci]

### 🚜 Refactor

- Standardize the headers used to check an email status
- Remove prisma client adapter

### 📚 Documentation

- Include the tracing level for the profile fetching from request cascade

## [6.6.0] - 2025-01-07

### 🚀 Features

- Apply the new profile filtering validation
- Wip - review the guest system

### 🐛 Bug Fixes

- Fix the permissioning system

### 💼 Other

- 6.5.0 → 6.6.0 [skip-ci]

## [6.5.0] - 2025-01-02

### 🚀 Features

- Turn the cert and key pem loading to use secret-resolver

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
- Expose the x-mycelium-request-id to the gateway user
- *(gateway-manager/service)* Implements the service listing for gateway managers
- *(user-account-creation)* Include a email notification to the new account creation workflow

### 🐛 Bug Fixes

- Fix the endpoints security definition
- Ensure the downstream service secrets to be removed from the gateway request
- Include additional checks to allow routing to insecure downstream paths only if explicitly informed by users
- Review the licensed resources filtering from database
- Fix the parsing and verification of connection strings not working
- Set the utoipa redoc environment variable on the main file of the api port
- Fix the webhook dispatching to decrypt secrets befhre send request to the incoming route

### 💼 Other

- 6.2.0 → 6.3.0 [skip-ci]

### 🚜 Refactor

- Rename the use-cases to mirror the application roles
- Move the match-forward-address to the api-port-router functionality
- Rename the standard folder to role-scoped in api prot
- Refactor the azure provider model to include new functionalities
- *(secret-dto)* Move the webhook secret dto to a independent dto named http-secret
- Refactor the route match use-case to use a correct base response from mycelium
- Move the match forward address use-case to the gateway use-cases group
- Remove the role submodule and move chindren modules to the root parent

### 📚 Documentation

- Initialize the redoc documentation elements
- Indicate a todo task to the redoc documentation
- Include openai specification for azure and google endpoints

## [6.2.0] - 2024-12-01

### 🚀 Features

- Review the full api documentation and endpoints locations to improve the development experience and usability

### 🐛 Bug Fixes

- Replace the myc path url by adm

### 💼 Other

- 6.1.0 → 6.2.0 [skip-ci]

### 🎨 Styling

- Upgrade the redoc base styles

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
- Replace the tenant id from the api route to use a x-mycelium-tenant-id header
- Refactor all routes to be more consistent
- Refactor the no-role guest endpoint to the new service route group

### ⚡ Performance

- Improve the profile injection on internal roles to filter roles before send by downstream requests

## [6.0.0] - 2024-11-13

### 🚀 Features

- Upgrade the internal authentication flow to generate simple authentication tokens from mycelium
- Improve the mycelium native auth to allow logins
- Wip - initialize the migration from activation url to numeric token on create new user accounts
- Improve the user activation code
- Implements the base for opentelemetry in lycelium
- Replace logs from the core use-cases by tracing
- Implements the password recovery flow
- Wip - do implement the new tenant based accounts management
- Implements the tenant management endpoints
- Implements the tenant-owner endpoints
- Implements the tenant-manager endpoint related elements
- Replace the smtp direct sender by a scheduler sender
- Wip - implements the guest role children insertion and deletion features
- Implemens the children guest role management endpoints
- Implements the route level filtration by role
- Implements the route filtration by permissioned roles
- Implements the guest-to-children-account use-case as a api port endpoint
- Implements the connection string elements to generate service tokens
- Implements the prisma adapter to create new connection string tokens and remove unused imports from native-errors in endpoints
- Implements the token creation endpoint of guest-manager
- Implements the prisma and api injectors for token fetching module
- Implements the totp initial steps for the otp registration
- Implements the totp activation
- Implements the two setp login using totp flow
- Implements the totp disable
- Upgrade the azure authorization flow in replacement to the remote check

### 🐛 Bug Fixes

- Move the email template location and fix the email verification code generation
- Move the tracing initialization to move it to the root of the main api function
- Fix return tyoe of login function
- Re-introduce the staff endpoints
- Reintroduce the system and users management endpoints
- Remove google and azure endpoints from logging ignore rules
- Fix the email consumption queue processor
- Replace the guesting email template element to use the tera template
- Fix the guest process
- Replace the profile injection to responds with unauthorized instead of forbidden
- Fix staff endpoints to upgrade and downgrade accounts
- Fix the email processing counter and fix the child role invitation use-case to avoid guest to different roles that the target one
- Allow users and staffs to use role protected routes
- Include the redaction function on get webhook from database
- Fix the webhook lifecycle to live as a more verbose to the final users
- Review the account list method to allow filter directly by account-type

### 💼 Other

- 5.0.8 → 6.0.0 [skip-ci]

### 🚜 Refactor

- Move the session_token to account-life-cycle module
- Rename the user and subscription manager roles
- Move the guest to subscription-manager instead to guest-managers
- Reactivate guests endpoints and move it to the subscription-manager endpoints group
- Turn tenant endpoints of manager api module a file
- Refactor endpoints to use the standard error code wrappers
- Move otel acessory functions from main api port file to a dedicated otel file
- Rename all json-error occurrent by http-json-error crate
- Refactor the mycelium smtp to be a general purpose notifier
- Move the guest-role to a dedicated dto module
- [**breaking**] Refactor the permissions to be a integer with read write and read-write options only
- Rename user to users and subscription by subscriptions as default actors and mirror to dependent elements
- Move the email sender to a dedicated module shared between use-cases and create a new mapped-error-to-http-response mapping handled
- Rename the token generator for the account associated connection string use-case
- Refactor webhooks to follow de main stream format widely used in web applications
- Refactor providers to standardize modules

### 📚 Documentation

- Include open-api absent structs from the user endpoint group

## [5.0.8] - 2024-04-25

### 🐛 Bug Fixes

- Wip - improve the google authentication checking logs and the api port logs to allow better debug

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

## [5.0.4] - 2024-04-09

### 🐛 Bug Fixes

- Inplements the google checks for oauth2 token online

### 💼 Other

- 5.0.3 → 5.0.4 [skip-ci]

## [5.0.3] - 2024-04-08

### 💼 Other

- 5.0.2 → 5.0.3 [skip-ci]

## [5.0.2] - 2024-03-21

### 🐛 Bug Fixes

- Fix actix-web corst to return specifig headers into responses

### 💼 Other

- 5.0.1 → 5.0.2 [skip-ci]

## [5.0.1] - 2024-03-11

### 🐛 Bug Fixes

- Rename gateway request estractors of the injected profile

### 💼 Other

- 5.0.0 → 5.0.1 [skip-ci]

## [5.0.0] - 2024-03-09

### 💼 Other

- Include http-tools in dockerfile
- 4.16.0 → 5.0.0 [skip-ci]

### ⚡ Performance

- *(licensed-resources)* [**breaking**] Replace the licensed resources fetching to use a view instead to perform multiple joins to fetch licenses contents

## [4.16.0] - 2024-02-26

### 💼 Other

- 4.15.3 → 4.16.0 [skip-ci]

## [4.15.3] - 2024-02-22

### 💼 Other

- 4.15.2 → 4.15.3 [skip-ci]

### 🚜 Refactor

- Move the api to a backward directory given the absence of the base myc-http-tools library
- Move the mycelium-http-tools to a dedicated module and kept the api related middleware elements to the api port module
- Move the sql adapters used during the profile extraction from requests of the api middleware to the own funciton that execute the action

## [4.15.2] - 2024-02-21

### 💼 Other

- 4.15.1 → 4.15.2 [skip-ci]

### 🚜 Refactor

- Move the myc-http-tools to a dedicated package

## [4.15.1] - 2024-02-15

### 🚀 Features

- Implements the api port to interact with the propagation use-case
- *(subscription-account)* Implements a use-case and endpoint to update accounts name and flags
- Implements the new base package to replace the clean-base package
- Upgrade webhook propagation functionalities to passthrough the bearer token together request
- Include default actors as an public object into the myc-http-tools
- Implements the slug name to allow accounts renaming without rename
- Implements the tags creation endpoint entities and use-cases
- Implements a reelated account enumerator that allow users to check permissions for a specified account or itself has privileged permissions

### 🐛 Bug Fixes

- Fix role and guest-role endpoints to use correct verbs and rest syntax
- Fix guest-role url parameter wrong expecting the role id as url parameter
- Fix guest-role endpoints params to use from body instead header
- Fix the argument delivery on guest-role endpoints
- Include role creation models into openapi definitions
- Fix the subscription account creation elements to allow propagation of default-users accounts
- *(subscription-account)* Include the subscription-accounts name and flags updating endpoint
- *(subscription-account-propagation)* Fix the subscription account propagation use-cases
- Fix the subscription accounts search to include tag search and a case insensitive search including uuid targeted search
- Orient all internal paths dependencies to the project path instead to use directly into the workspace

### 💼 Other

- Fix deprecation dependency
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

### 🐛 Bug Fixes

- Fix the error handler on try to create existing users

### 💼 Other

- 4.7.3 → 4.7.4 [skip-ci]

## [4.7.3] - 2023-12-17

### 💼 Other

- 4.7.2 → 4.7.3 [skip-ci]

## [4.7.2] - 2023-12-17

### 🐛 Bug Fixes

- *(default-account-creation)* Include identity extraction from request token on create default accounts

### 💼 Other

- 4.7.1 → 4.7.2 [skip-ci]

## [4.7.1] - 2023-12-17

### 🐛 Bug Fixes

- *(default-user-creatio9n)* Fix the absence of check of user token during user creation on use a third party provider

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

- Implements the configuration loading from environment already
- Implements the collection of secret values from environment instead of to use hardcoded configurations

### 💼 Other

- 4.5.1 → 4.6.0 [skip-ci]

## [4.5.1] - 2023-12-04

### 💼 Other

- Upgrade docker build files
- 4.5.0 → 4.5.1 [skip-ci]

## [4.5.0] - 2023-12-04

### 🚀 Features

- Implements the auxiliary endpoints

### 💼 Other

- 4.4.0 → 4.5.0 [skip-ci]

### 🚜 Refactor

- Move url groups api enumerator to a higher level inside the endpoints module

## [4.4.0] - 2023-12-04

### 🚀 Features

- Refactores standard and managers endpoints to mirror the new actors system

### 💼 Other

- 4.3.0 → 4.4.0 [skip-ci]

### 🚜 Refactor

- Turn default-user endpoints and apis to mirrir the new default system actors

## [4.3.0] - 2023-12-03

### 🚀 Features

- Implements new notifications and improve the accounts creation flow

### 💼 Other

- 4.2.0 → 4.3.0 [skip-ci]

## [4.2.0] - 2023-10-25

### 🚀 Features

- Upgrade prisma adapter user model to include providers as options
- Finish implementation of the user and account registrations in two independent steps
- Turn accounts creation process to possible without user registration
- Wip - start implementation of the session token management during users accounts lyfe cycle
- Wip - implements the config manager module
- Wip - implements the config manager module
- Migrate session-token management from redis to postgres
- Implements configuration passthrough from api port to another application layers
- Upgrade auth models tobe loaded from config file

### 🐛 Bug Fixes

- Fix the user fetching and registration adapters to include and omit password informations and init creation of users endpoints
- Fix google oauth configs wrong written
- Fix app configuration at the api port
- Remove unused commitizen from redis cardo toml

## [4.1.1] - 2023-09-19

### 🚀 Features

- Upgrade router to allow http2 service as downstream url for apis management

### 🐛 Bug Fixes

- Upgrade account creation use-cases to include or not profile information during accounts initializations

### 💼 Other

- Synchronize package sub versions
- Upgrade from 3 to 4 the major package version
- Partial increment package versions
- 4.1.0 → 4.1.1 [skip-ci]

### 🚜 Refactor

- Remove unused loggings from router

## [4.0.0] - 2023-09-07

### 🚀 Features

- Wip - upgrade ports to work with with webhooks
- Implements the webhooks creation and deletion usecases adapters and ports
- [**breaking**] Upgrade the account model to include multiple owners allowing to work with multi-user accounts with the same real world identity

### 🐛 Bug Fixes

- Extend previous commit

### 🚜 Refactor

- Split endpoints submodules to dedicated files
- Make stuff changes in router

## [3.0.1] - 2023-07-29

### 💼 Other

- 3.0.0 → 3.0.1 [skip-ci]

### ⚡ Performance

- Move user creation of the account creation process to a transaction into the account creation

## [3.0.0] - 2023-06-18

### 💼 Other

- 2.0.0 → 3.0.0 [skip-ci]

## [2.0.0] - 2023-06-18

### 💼 Other

- 1.0.0 → 2.0.0 [skip-ci]

### ⚙️ Miscellaneous Tasks

- Manual upgrade all versions of the mycelium package

## [1.0.0] - 2023-06-18

### 🚀 Features

- Implements the account-creation endpoint into the api port and their dependencies
- Implements the account updating prisma repository
- Implements guest-role deletion repository
- Implements the guest-role fetching prisma adapter
- Implements the guest-role updating prisma repository
- Implements the role deletion prisma repository
- Implements the role registration prisma repository
- Implements the role fetching prisma repository
- Implements the role updating prisma adapter
- Include all adapters as modules and into the shaku configuration to be injected at the application runtime
- Implements the token validation data repositoryes and dependencies
- Include redis adapters into the api port module
- Implements the token cleanup entities and redis adapters and prepare injection modules into api ports
- Update all package dependencies to use unsufixed dtos and update fetch profile use-case to include new arguments
- Update the profile fetch api response
- Include profile response elements as the exportation elements of the api-public port
- Update public elements of the myc-api library to include profile use-case dtos and update sub-packages versions
- Create endpoint to update account name
- Implement account endpoints of the managers api group
- Refacror the manager endpoints to split routes by group
- Implements manager role endpoints
- Imlement endpoints for token management by service users
- Update utoipa path params documentation of all already implemented endpoints
- Implement endpoints from shared use-cases group to manager endpoints group
- Creeate staff endpoints for accounts management
- Update profile extractor to validate token
- Collect the application name from environment
- Implement endpoints to get and list subscription accounts
- Implements entpoint to fetch guest-users using the subscription account id
- Implements list methods for role and guest-roles
- Create a new licensed-resources adapter to replace the guest-roles adapter used during profiles fetching
- Split fetch profile pack use-case in two sepparate use-cases and create a new default-users endpoint to provide profiles
- Start creation of the api gateway
- Create api gateways functionalities to turn mycelium independent
- Update extractor and credentials checker to work around profiles instead profile-packs
- Create methods to fetch user information from ms graph using me route
- Implements the second step for identity checking on fetch profile from the api request
- Implements all features to list accounts by type and include archivement options to database
- Implements the endpoint to get all guests of a default account
- Upgrade prisma client to reduce the build size of prisma installation
- Implements endpoint to change account archival status
- Wip - implements swagger elements to serve the project apis documentations
- Remove oauth2 checks from the api
- Remove authorization codes from the api configuration module
- Convert the account type from the managers account list to a flag indicating if only subscription account should be fetched
- Implements the verbose status notation to turn the status interpretation from tags to text
- Replace the accounts filtration from using flags to use accounts verbose statuses
- Implements the uninvite system for guests
- Implements the unapproval use-cases and appropriate endpoints
- Implement methods to update the guest-role of an existing guest
- Replace all manual error handling on use mapped-errors to infalible implementation
- Wip - start the implementation of the google identity checking flux
- Upgrade version of the clean-base error handlers at mycelium
- Implement use-cases to manage error codes
- Implements the api endpoints for error-codes management
- Remove health checks from the main api logger
- Include the google oauth2 flow for authorization-code flow usage
- Wip - create config and handlers for azure oauth2 authorization-code flow
- Include user names into the profile object
- Fix the wrong unwrap occurred on get used identity GatewayProfileData
- Update user related dtos to include partial equals as derive and upgrade get ids elements of profile
- [**breaking**] Wip - implements the app interface for management and create the commitizen file for auto versioning

### 🐛 Bug Fixes

- Update the response body and schemas from service and staff openapi
- Stamdardize all application routes and fix actix-web endpoints configurations
- Remove unnecessary loggings from debug
- Fix response of the subscription accounts fetching from created to ok
- Create a constructor method for json-error instead of the direct instance creation
- Fix the guest accounts listing route to remove extra backslash
- Fix guest-role use-case to include role id as optional filtration argument
- Remove permissions settings form the guest-roles creation url
- Fix bug into the collection of profile during collection of the guest accounts
- Update profile extractor message
- Upgrade version of clean-base and utoipa to fix downgrade dependencies
- Fix the profile validation and remove all code from token checking
- Replace path from context-path at the openapi docs of each endpoins available in the mycelium
- Create an option to extract profile from header instead of try to extract directely from request
- Remove unused dependencies from mycelium api
- Include activation status change endpoints to the account endpoints set
- Include permission type enumerator definition on the default-users endpoints openapi definition
- Replace body param of query guest to query param instead
- Replace utoipa definitions of manager guest url to use params as query instead of body params
- Upgrade the response status when an uninvitation event is called from manager endpoint
- Resolve the error on uninvite guest to account
- Create a code field to present the concatenated prefix and error_number as the code
- Replace all 404 responses of valid request by 204 ones
- Fix the decoding of headers parsing

### 💼 Other

- Change the package name from mycelium to myc to avoid incompatibility with another existing crate package
- Replace the myc-core from the local github to the crate directory
- Update the project version and add badges to the main readme file
- Update all package versions
- Upgrade overall package versions before tag

### 🚜 Refactor

- Move ports to a dedicated directory
- Wip - move infra and smtp adapters to the adapters feature
- Finish the migration of the smtp adapters to the adapters feature
- Ungroup adapters by role
- Update api ports to export adapters after the latest refactoring
- Fix paths of myc-core entities imports after upgrade the myc-core version
- Refactore adapter imports and move modules to a single file
- Re-export adapters to the root reposotories module
- Remove unused horizintal marker from the api config
- Update ports to adequate to the use-cases refactoring from the previous commits
- Move account approval and status changes to the manager use-cases group
- Rename the simple forwarding error from gateway to gateway error
- Replace the default error imports from clean-base in use-case module to the new error factory import

### 📚 Documentation

- Includes package metadata in all subpackages of this project
- Update packages category slugs to match valid crate categories
- Update myc-api port readme to instruct the development mode run
- Include documentation of the check-credentials main use-case
- Documenting the post endpoints of the manager api group

<!-- generated by git-cliff -->
