# Changelog

All notable changes to this project will be documented in this file.

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

### 🐛 Bug Fixes

- *(create-healthcheck-partition)* Fix the create_healthcheck_partition sql funciton to create partitions

### 💼 Other

- 7.8.1 → 7.8.2.dev1 [skip-ci]
- 7.8.2.dev1 → 8.0.0.dev2 [skip-ci]
- Upgrade version scheme to adequate to semver

### ⚙️ Miscellaneous Tasks

- Fix version of commitizen

## [7.8.1] - 2025-04-22

### 🐛 Bug Fixes

- Replace the ensure daily partition by a secondary access function to avoid higher level accessions

### 💼 Other

- 7.8.0 → 7.8.1 [skip-ci]

## [7.8.0] - 2025-04-22

### 🚀 Features

- Create sql model for healthcheck-logs data storage
- *(multihost-support)* Add support for multiple hosts for downstream services
- Wip - implements in memory tracking for health check metrics
- Implements base models for healthcheck-logs in diesel adapter
- Finish the implementation of the healthcheck-logs registration in database
- Improve the tools discovery endpoint to include more information about context and capabilities

### 🐛 Bug Fixes

- Include the error-message field in sql model for healthcheck

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

- *(tenant-details)* Include details of owners tags and manager account to the tenant details when called by tenant-managers

### 💼 Other

- 7.7.9 → 7.7.10 [skip-ci]
- 7.7.10 → 7.7.11 [skip-ci]
- 7.7.11 → 7.7.12 [skip-ci]

## [7.7.9] - 2025-03-29

### 💼 Other

- 7.7.8 → 7.7.9 [skip-ci]

## [7.7.8] - 2025-03-29

### 💼 Other

- 7.7.7 → 7.7.8 [skip-ci]

## [7.7.7] - 2025-03-28

### 🐛 Bug Fixes

- Include an additional step during the tenants management account creation to register the manager account on tenant

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
- Implements an endpoint to serve tenant information for tenant-manager accounts
- Do implements pagination when list guest users on account

### 🐛 Bug Fixes

- Populate the created-by field during the creation of a connection between two guest-roles
- Populate the created-by field during the creation of a connection between two guest-roles

### 💼 Other

- 7.6.0 → 7.7.0 [skip-ci]

## [7.6.0] - 2025-03-14

### 🚀 Features

- Implements a system flag in guest-roles to indicate roles restricted to system accounts

### 💼 Other

- Move the up sql file to the diesel adapter
- 7.5.11 → 7.6.0 [skip-ci]

## [7.5.11] - 2025-03-10

### 💼 Other

- 7.5.10 → 7.5.11 [skip-ci]

## [7.5.10] - 2025-03-10

### 🚀 Features

- Remove the redis dependency from notifier system and replace by postgres dependency

### 💼 Other

- 7.5.9 → 7.5.10 [skip-ci]

## [7.5.9] - 2025-03-10

### 💼 Other

- 7.5.8 → 7.5.9 [skip-ci]

## [7.5.8] - 2025-03-06

### 🐛 Bug Fixes

- Upgrade account list to allow non tenant requests and refine the account filtering based on the user roles
- Replace licensed resources parsing during their loading on diesel adapter
- Include the guest role id during the guest user connection to account

### 💼 Other

- 7.5.7 → 7.5.8 [skip-ci]

## [7.5.7] - 2025-03-04

### 💼 Other

- 7.5.6 → 7.5.7 [skip-ci]

## [7.5.6] - 2025-03-04

### 💼 Other

- 7.5.5 → 7.5.6 [skip-ci]

## [7.5.5] - 2025-03-01

### 🐛 Bug Fixes

- Turn webhooks paginated

### 💼 Other

- 7.5.4 → 7.5.5 [skip-ci]

## [7.5.4] - 2025-02-26

### 🚀 Features

- Do implements the public tenant fetcher

### 💼 Other

- 7.5.3 → 7.5.4 [skip-ci]

## [7.5.3] - 2025-02-19

### 💼 Other

- 7.5.2 → 7.5.3 [skip-ci]

## [7.5.2] - 2025-02-19

### 💼 Other

- 7.5.1 → 7.5.2 [skip-ci]

## [7.5.1] - 2025-02-18

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

### 🐛 Bug Fixes

- Remove log crate from the project

### 💼 Other

- 7.1.0 → 7.2.0 [skip-ci]

## [7.1.0] - 2025-01-31

### 🚀 Features

- Implements diesel models to mirror the sql implementation for the prisma adapter
- Create the basis for the diesel adapter
- Implements the account repository for diesel
- Implements the error code diesel repository and initialize modules for other entities
- Implemente diesel adapter to guest_role
- Implements the guest_user diesel adapter
- Implements the tenant adapters for diesel
- Implements the webhook diesel adapters
- Implements methods for tenant tag for diesel adapter
- Implements the users adapter for prisma
- Implements the token adapter for diesel
- Implements the profile adapter for diesel
- Implements the licensed-resources diesel adapter
- Migrate all prisma dependencies to diesel
- Review the weghook trigger names to improve the user understanding of their goal
- Wip - do implements the asynchronous dispatching of webhooks
- Wip - implements the async dispatcher functionality

### 🐛 Bug Fixes

- Include the meta field at the account model
- Fix the accout fetching adapter for prisma
- Fix the diesel injection module to avoid modules implementation expositions
- Replace diesel uuid in models and repositories by string
- Fix the profile fetching diesel query
- Fix the user token invalidation on create a new one
- Fix the totp lifecycle
- Fix the tenant fetching to migrato to native orm diesel query
- Fix the tenant fetching process
- Fix the tenant operations related to the diesel database engnie
- Improve the tenant fetching and account fetching to adequate to the expected behaviour already validated with prisma
- Review the error code life cycle
- Wip - fix the guest roles diesel orm functionalities
- *(subscription-accounts)* Fix the subscription accounts related operations
- Review the guest roles related operations
- Fix the guest to children account
- Fix the meta endpoints for account meta management
- Fix the webhook async dispatch to avoid updates of the payload package and mirror important changes to database

### 💼 Other

- Include diesel adapter as a commitizen tracked file
- 7.0.0 → 7.1.0 [skip-ci]

### 🚜 Refactor

- Move the account-tag to a dedicated module
- Remove prisma client adapter
- Move tracing and async dispatchers to dedicated modules

### 📚 Documentation

- Include the tracing level for the profile fetching from request cascade

<!-- generated by git-cliff -->
