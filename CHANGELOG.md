## v4.7.14 (2024-01-08)

### Fix

- fix action type

## v4.7.13 (2024-01-08)

### Fix

- implementing serialization and deserialization into the permissions action type enumerator

## v4.7.12 (2024-01-08)

### Fix

- include role creation models into openapi definitions

## v4.7.11 (2024-01-06)

### Fix

- fix the argument delivery on guest-role endpoints

## v4.7.10 (2024-01-05)

### Fix

- fix guest-role endpoints params to use from body instead header

## v4.7.9 (2024-01-05)

### Fix

- fix guest-role url parameter wrong expecting the role id as url parameter

## v4.7.8 (2024-01-05)

### Fix

- fix role and guest-role endpoints to use correct verbs and rest syntax

## v4.7.7 (2023-12-23)

### Fix

- upgrade commitizen config file to upgrade the version from workspace file

## v4.7.6 (2023-12-23)

### Refactor

- refactor the cargo dependencies to import shared dependencies from the workspace

## v4.7.5 (2023-12-17)

### Fix

- fix the response from account creation to return a 409 code if account exists

## v4.7.4 (2023-12-17)

### Fix

- fix the error handler on try to create existing users

## v4.7.3 (2023-12-17)

### Fix

- move the email notifier on generate tokens to be collected form environment

## v4.7.2 (2023-12-17)

### Fix

- **default-account-creation**: include identity extraction from request token on create default accounts

## v4.7.1 (2023-12-16)

### Fix

- **default-user-creatio9n**: fix the absence of check of user token during user creation on use a third party provider

## v4.7.0 (2023-12-14)

### Feat

- implements webhook updating and listing

## v4.6.1 (2023-12-07)

### Refactor

- rename accessor method of env-or-value from get to get-or-error to indicate that the mathod returns a result

## v4.6.0 (2023-12-06)

### Feat

- implements the collection of secret values from environment instead of to use hardcoded configurations
- implements the configuration loading from environment already

## v4.5.1 (2023-12-04)

## v4.5.0 (2023-12-04)

### Feat

- implements the auxiliary endpoints

### Refactor

- move url groups api enumerator to a higher level inside the endpoints module

## v4.4.0 (2023-12-04)

### Feat

- refactores standard and managers endpoints to mirror the new actors system

### Refactor

- move error codes to system-manager role use-cases group
- move manager use-cases to dedicated actors folders
- turn default-user endpoints and apis to mirrir the new default system actors

## v4.3.0 (2023-12-02)

### Feat

- implements new notifications and improve the accounts creation flow

## v4.2.0 (2023-10-25)

### Feat

- upgrade auth models tobe loaded from config file
- implements configuration passthrough from api port to another application layers
- migrate session-token management from redis to postgres
- wip - implements the config manager module
- wip - implements the config manager module
- wip - start implementation of the session token management during users accounts lyfe cycle
- start implementation of the config manager
- turn accounts creation process to possible without user registration
- create is-principal field to indicate users which created the accounts
- finish implementation of the user and account registrations in two independent steps
- implements the email check function to start the proccess of authentication
- upgrade prisma adapter user model to include providers as options

### Fix

- fix app configuration at the api port
- fix google oauth configs wrong written
- fix the user fetching and registration adapters to include and omit password informations and init creation of users endpoints
- upgrade staff account creation flow to include password and hash into data persistence modules
- include error handling if some exception occurred during the user registration

## v4.1.1 (2023-09-19)

### Fix

- fix prisma adapter to deals with the profile field as private
- turn provider and their password hash and salt as private fields

## v4.1.0 (2023-09-19)

### Feat

- move new method of user new_with_provider method allow the provider field to be required during default object creation
- create base attributes to implement password checks to users
- wip - start creation of the token management elements
- wip - start creation of the default account management
- upgrade router to allow http2 service as downstream url for apis management

### Fix

- upgrade user model to remove password hash and salt information before serialize user object
- upgrade account creation use-cases to include or not profile information during accounts initializations
- remove non domain logic from the session token domain dto
- finish use-cases related to session token registration and expiration
- fix account and user prisma adapters to deals with user provider models

### Refactor

- move all entities to dedicated folders mirriring their role in the application
- remove unused loggings from router
- rename config loading use-case to indicate that config is loaded from yaml file not json
- rename account propagation response during webhook actions

## v4.0.0 (2023-07-29)

## v3.0.2 (2023-07-29)

### Fix

- remove app from the main package

## v3.0.1 (2023-07-29)

### Perf

- move user creation of the account creation process to a transaction into the account creation

## v3.0.0 (2023-06-18)

### BREAKING CHANGE

- main

### Feat

- remove the include-itself parameter of the profile functions that check privilegees

## v2.0.0 (2023-06-18)

## v1.0.0 (2023-06-18)

### BREAKING CHANGE

- main

### Feat

- wip - implements the app interface for management and create the commitizen file for auto versioning

## v34.0-beta (2023-06-18)

### Feat

- turn default include-itself parameter of profile as false

## v33.0-beta (2023-06-18)

### Feat

- update user related dtos to include partial equals as derive and upgrade get ids elements of profile

## v32.0-beta (2023-06-06)

### Fix

- fix the decoding of headers parsing

## v31.0-beta (2023-06-06)

### Feat

- Fix the wrong unwrap occurred on get used identity GatewayProfileData

## v30.0-beta (2023-06-06)

### Feat

- include user names into the profile object

## v29.0-beta (2023-06-06)

### Feat

- wip - create config and handlers for azure oauth2 authorization-code flow
- include the google oauth2 flow for authorization-code flow usage
- remove health checks from the main api logger

## v28.0-beta (2023-05-18)

### Feat

- create methods to check ids permissinos with error status if no licensed ids were found

### Fix

- replace all 404 responses of valid request by 204 ones
- replace the is-internal from is-native flag evaluation on try to registre new errors
- create a code field to present the concatenated prefix and error_number as the code
- resolve the error on uninvite guest to account
- **guest-user-deletion**: update the guest-user-deletion prisma repository to delete guest from guest role id instead of the guest user-id
- upgrade the response status when an uninvitation event is called from manager endpoint
- include account flags during the creation of the account

## v27.0-beta (2023-04-25)

### BREAKING CHANGE

- main

### Feat

- upgrade prisma version
- implements the batch creation of the native error persistence
- implements the api endpoints for error-codes management
- implement use-cases to manage error codes
- implements the error code prisma crud and include message code to prisma client
- implements entities for error code crud management
- upgrade version of the clean-base error handlers at mycelium
- wip - start the implementation of the google identity checking flux
- create the error code dto to manage code data
- upgrade all gateway use-cases and some remaining use-cases from roles group to use infallibles
- replace all manual error handling on use mapped-errors to infalible implementation

### Fix

- replace utoipa definitions of manager guest url to use params as query instead of body params
- replace body param of query guest to query param instead
- set checked as true during the creation of subscription accounts
- remove full file path from the auto-generated prisma code

### Refactor

- replace the default error imports from clean-base in use-case module to the new error factory import

## v26.0-beta (2023-04-04)

### Feat

- implement methods to update the guest-role of an existing guest
- include the account name and guest-role name into the licensed resources object
- include the possibility to jump from active users to archived users in managers use-cases
- implements the unapproval use-cases and appropriate endpoints
- implements the uninvite system for guests
- replace the accounts filtration from using flags to use accounts verbose statuses
- implements the verbose status notation to turn the status interpretation from tags to text
- convert the account type from the managers account list to a flag indicating if only subscription account should be fetched
- remove authorization codes from the api configuration module
- remove oauth2 checks from the api
- wip - implements swagger elements to serve the project apis documentations

### Fix

- upgrade the status state checker to allow users to migrate to archived from inactive users
- include an and statement during the filtration of the accounts

### Refactor

- rename the simple forwarding error from gateway to gateway error

## v25.0-beta (2023-02-28)

### Feat

- implements endpoint to change account archival status

### Fix

- include permission type enumerator definition on the default-users endpoints openapi definition
- include activation status change endpoints to the account endpoints set
- fix bug during the approval or activation status change of an account

### Refactor

- move account approval and status changes to the manager use-cases group

## v24.0-beta (2023-02-27)

### Feat

- upgrade prisma client to reduce the build size of prisma installation
- implements the endpoint to get all guests of a default account
- implements all features to list accounts by type and include archivement options to database

### Fix

- remove unused dependencies from mycelium api

## v23.0-beta (2023-02-15)

### Feat

- implements the second step for identity checking on fetch profile from the api request

## v22.0-beta (2023-02-14)

### Fix

- fix bug repository to fetch licensed ids

## v21.0-beta (2023-02-14)

### Fix

- **licensed-resources-fetching**: replace the licensed resources fetching from guest-users to guest-users-on-accoun data source

## v20.0-beta (2023-02-13)

### Fix

- create an option to extract profile from header instead of try to extract directely from request
- fix bug during the creation of the guest users

## v19.0-beta (2023-02-10)

### Feat

- create methods to fetch user information from ms graph using me route
- update extractor and credentials checker to work around profiles instead profile-packs
- create api gateways functionalities to turn mycelium independent
- start creation of the api gateway

### Fix

- replace path from context-path at the openapi docs of each endpoins available in the mycelium
- fix the profile validation and remove all code from token checking

## v18.0-beta (2023-01-30)

### Fix

- upgrade version of clean-base and utoipa to fix downgrade dependencies

## v17.0-beta (2023-01-30)

### Feat

- split fetch profile pack use-case in two sepparate use-cases and create a new default-users endpoint to provide profiles

### Fix

- change log type from token deregistration from warn to debug

## v16.0-beta (2023-01-23)

### Feat

- include an optional parameter to allow exclude the own id from from the licensed profiles list

## v15.0-beta (2023-01-22)

### Feat

- create a new licensed-resources adapter to replace the guest-roles adapter used during profiles fetching

### Fix

- remove the full file path from the prisma generation file

## v14.0-beta (2023-01-20)

### Fix

- update profile extractor message
- include a debug log after build and collect profile
- fix bug into the collection of profile during collection of the guest accounts
- increase permissions of the myc-service-role to allow data insertion and reading in postgres development database

## v10.4-beta (2023-01-17)

## v10.3-beta (2023-01-17)

### Fix

- remove permissions settings form the guest-roles creation url
- include a string parser on the permissions-type enumerator to allow parse permissions from api request

## v10.2-beta (2023-01-17)

### Feat

- implements list methods for role and guest-roles
- implements entpoint to fetch guest-users using the subscription account id

### Fix

- fix regex validation for email string parsing and include new tests for it
- fix guest-role use-case to include role id as optional filtration argument
- replace the guest-role id by the role id wrong send during the subscription guest list fetching operation
- fix the guest accounts listing route to remove extra backslash

## v10.1-beta (2023-01-13)

### Fix

- create a constructor method for json-error instead of the direct instance creation
- fix response of the subscription accounts fetching from created to ok

## v10.0-beta (2023-01-13)

### Feat

- implement endpoints to get and list subscription accounts
- change redis donnector to collect redis connection string trom the environment

### Fix

- fix the account-type parent type from id to record to avoid execution-error on execute the guest-user use-case
- include owner and account-type as related elements on try to fetch accounts

## v9.0-beta (2023-01-06)

### Feat

- update descriptions of default accounts
- collect the application name from environment
- update profile extractor to validate token
- creeate staff endpoints for accounts management
- implement endpoints from shared use-cases group to manager endpoints group
- update seed staff creation to include is-manager flag at the created account
- update utoipa path params documentation of all already implemented endpoints
- imlement endpoints for token management by service users
- implements manager role endpoints
- refacror the manager endpoints to split routes by group
- implement account endpoints of the managers api group
- create endpoint to update account name
- update the cli port that creates the seed staff account

### Fix

- remove unnecessary loggings from debug
- stamdardize all application routes and fix actix-web endpoints configurations
- update the response body and schemas from service and staff openapi
- update visibility of the default account creation use-case to be accessible only from crates
- fix the privilegies misverification on the account activation use-case
- fix the error returned by adapters during the creation of the user and account

### Refactor

- move the upgrade and downgrade actions from shared to staff use-cases group

## v8.0-beta (2023-01-03)

### Feat

- update public elements of the myc-api library to include profile use-case dtos and update sub-packages versions
- include profile response elements as the exportation elements of the api-public port
- update the profile fetch api response
- update all package dependencies to use unsufixed dtos and update fetch profile use-case to include new arguments
- update the profile fetch use-case to include the validation token creation in the process
- limit the visivility of the token-expiration-time config to crate

### Refactor

- remove the dto notation from the domain dtos

## v7.1-beta (2022-12-31)

## v7.0-beta (2022-12-31)

### Feat

- implements the token cleanup entities and redis adapters and prepare injection modules into api ports
- update registration and de-registration adapter to group tokens by date allowing cleanup every day

## v6.0-beta (2022-12-30)

### Feat

- include redis adapters into the api port module
- implements the token validation data repositoryes and dependencies
- implements the use-case and entities and their dtos to perform token registration and deregistration

## v5.0-beta (2022-12-27)

### Refactor

- update ports to adequate to the use-cases refactoring from the previous commits
- refactore use-cases to re-export main case functions into the parent modules directelly

## v4.0-beta (2022-12-27)

### BREAKING CHANGE

- main

### Feat

- include all adapters as modules and into the shaku configuration to be injected at the application runtime
- implements the role updating prisma adapter
- implements the role fetching prisma repository
- implements the role registration prisma repository
- implements the role deletion prisma repository
- implements the guest-role updating prisma repository
- implements the guest-role fetching prisma adapter
- implements guest-role deletion repository
- replace the guest-role deletion return type from dto to uuid
- implements the account updating prisma repository
- replace the id type of the get method by uuid

### Refactor

- remove unused horizintal marker from the api config
- re-export adapters to the root reposotories module
- refactore adapter imports and move modules to a single file
- fix paths of myc-core entities imports after upgrade the myc-core version
- re-export all entities from their parend module instead of publish their directly
- update api ports to export adapters after the latest refactoring
- ungroup adapters by role
- move all antities from core packate to the root entities folder instead of seggregate by role

## v3.0-beta (2022-12-27)

### Refactor

- remove the md extension from tthe license file

## v2.5-beta (2022-12-26)

### Feat

- update profile methods to get ids by permission to include a list of roles instead of a single role

## v2.4-beta (2022-12-25)

### Feat

- include methods to filter ids of accounts that profile has permissions
- implements the account-creation endpoint into the api port and their dependencies
- documenting the profile extractor of the public module and update the default profile key of the core module

### Refactor

- finish the migration of the smtp adapters to the adapters feature
- wip - move infra and smtp adapters to the adapters feature
- move ports to a dedicated directory

## v2.3-beta (2022-12-24)

### Feat

- include email-dto into the re-exports

## v2.2-beta (2022-12-24)

### Feat

- re-export elements from the myc-core to reduce the number of packages to install

## v2.1-beta (2022-12-23)

### Refactor

- rename the core package from myc to myc-core

## v2.0-beta (2022-12-23)

### Refactor

- move the cli port to a dedicated project
- move the api port to a dedicated submodle
- rename internal project folders from mycelium to myc

## v1.0-beta (2022-12-23)

### BREAKING CHANGE

- main

### Feat

- update the guest-user use-case to fetch profile from request and implement methods to extract it
- implements the adapter for guest-role registrations
- implements the account-type registration adapter
- implement endpoints to fetch profiles from email and update profile object to be more informative
- implements sql adapter to fetch profile from database
- update profile dto to include information from the subscription flags and other
- implements the use-case to fetch the user profile by email
- include an account validation method on guest use-case and move the seed staff account creatio to staff use-cases
- split application into api and cli and rename the seed staff creation function
- implements the use-case to guest a user to a specific role
- create actions create staff accounts and upgrade and downgrade account types
- create a use-case to start subscription accounts
- implement the use case to update role name and description
- implement use-case to allow users to update the own account name
- implements the user-registration adapter
- create the use-case to mark accounts as checked
- create use-case to activate and deactivate accounts
- mirroring the data model into the database model
- move the prisma module to a dedicated sub-project
- update the account type and dependencies to deal with the is-subscription account type
- replace the raw mappederrors by the specific errors functions
- implements the user role deletion and updating for manager users
- include the account id at the user-role object aht their use-case registration
- include privilege checking for the roles registration use-case
- implements the user-role registration use-case and dependencies
- create the basic entities for the application roles and the use-case to create an account with default account-type
- update the profile and user dtos to use a new email dto validation object instead of a raw string
- implement a url generator on guest and user-rule structs
- implements the url generator into accounts
- wip - initialize the basis for the application evolution
- update all dtos to work with parent and children representation of relations over pure types
- implements abstract methods for application management
- implements the initial structure and dtos
- initial commit

### Fix

- fix the guest-role registration adapter
- include a match validation during the email validation that uses regex
- fix the check on the guesting process which the subscription account were not correctely checked
- remove profile requirement from the seed staff account creation
- rename manager use-cases from simple role to guest-role to match the application rules
- turn account type flags as static booleans instead of optionals

### Refactor

- fix the header profile key in settings and rename the profile-extraction function to best denote their goal
- reduce the profile parsing on fetch it from database
- rename remove-guest use-case to uninvite-guest
- rename the create-account use-case to create-default-account to mirror the main goal of the use case that is create default accounts
- migrate the base objects to the agrobase-rs
- group use-cases by data afinity
- rename user-role entities to guest-roles
