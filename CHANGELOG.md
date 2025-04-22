## v7.8.1 (2025-04-22)

### Fix

- replace the ensure daily partition by a secondary access function to avoid higher level accessions

## v7.8.0 (2025-04-22)

### Feat

- improve the tools discovery endpoint to include more information about context and capabilities
- finish the implementation of the healthcheck-logs registration in database
- implements base models for healthcheck-logs in diesel adapter
- wip - implements in memory tracking for health check metrics
- create a script to backup and drop old logs from the health check system
- **multihost-support**: add support for multiple hosts for downstream services
- create sql model for healthcheck-logs data storage
- finish the migration of the lazy-static for in-memory storage of soutes to use shaku injection of in-memory data
- wip - replace the lazy-static implementation of the memory database by a shaku injectable module
- **tools**: wip - implements changes to serve discoverable services through endpoint

### Fix

- remove the partition creation part of the accessoru script for healthcheck logs management
- include the error-message field in sql model for healthcheck
- store drop partition operation result into a variable
- turn service id required field
- remove old module folder of the api-port submodule
- **tools**: reduce information of the services available during discoverability

## v7.7.13 (2025-04-01)

### Fix

- include a slug filtration during guest roles fetching

## v7.7.12 (2025-03-29)

### Fix

- use owner ids instead of the account id during filtration of the tenant details

## v7.7.11 (2025-03-29)

### Feat

- include the possibility to tenant-owners to request tenant details using the tenant-manager endpoint

### Fix

- **get-tenant-details**: handle get-ids-or-error result dispatch to avoid young return with error on check if user is tenant-manager
- **tenant-details**: include details of owners tags and manager account to the tenant details when called by tenant-managers

## v7.7.10 (2025-03-29)

## v7.7.9 (2025-03-29)

### Fix

- **profile**: reconigure permissions to allow greater or equal instead to equal only on check user profile permissions

## v7.7.8 (2025-03-29)

### Fix

- remove the read-write option from permissions to avoid ambiguous filtering of profile options

## v7.7.7 (2025-03-28)

### Fix

- include an additional step during the tenants management account creation to register the manager account on tenant

## v7.7.6 (2025-03-28)

### Fix

- freeze zip version on try to avoid error on utoipa-swagger-ui

## v7.7.5 (2025-03-27)

### Fix

- downgrade utoipa to avoid nine version error on build

## v7.7.4 (2025-03-27)

### Fix

- upgrade project dependencies including dependabot issues

## v7.7.3 (2025-03-26)

### Fix

- **dependabot**: introduce security fixes recommended by bependabot

## v7.7.2 (2025-03-26)

### Fix

- **utoipa-swagger-ui**: upgrade utoipa-swagger-ui version to 9

## v7.7.1 (2025-03-26)

### Fix

- **utoipa-swagger-ui**: include the reqwest feature flag in utoipa-swagger-ui to avoid to use curl during the swagger installation

## v7.7.0 (2025-03-26)

### Feat

- do implements pagination when list guest users on account
- implements an endpoint to serve tenant information for tenant-manager accounts
- implements a new endpoint to serve account details to the account owners from the beginners api group
- implements new created and updated fields and propagate this fields along the software stack
- implements new created and updated fields and propagate this fields along the software stack

### Fix

- populate the created-by field during the creation of a connection between two guest-roles
- populate the created-by field during the creation of a connection between two guest-roles
- **create-management-account**: set the is-detault flag before persist the new created account to the datastore
- **downgrade-account-privileges**: include a check to deny downgrade operations in non-self accounts
- **create-default-account**: upgrade the create-default-account use-case to generate the account slug from the user principal email
- **update-account-name-and-flags**: include a logic to avoid the updating of the account slug after update account name in update-account-name-and-flags use-case

## v7.6.0 (2025-03-14)

### Feat

- implements a system flag in guest-roles to indicate roles restricted to system accounts

## v7.5.11 (2025-03-10)

### Fix

- upgrade production configurations to load templates as a project artifact

## v7.5.10 (2025-03-10)

### Feat

- remove the redis dependency from notifier system and replace by postgres dependency

## v7.5.9 (2025-03-09)

### Feat

- implements a ping test to the email dispatcher initialization

## v7.5.8 (2025-03-06)

### Fix

- include the guest role id during the guest user connection to account
- replace licensed resources parsing during their loading on diesel adapter
- remove lock for subscription account on list guest to account
- upgrade account list to allow non tenant requests and refine the account filtering based on the user roles

## v7.5.7 (2025-03-04)

### Feat

- implements a guest-role fetching details for subscriptions managers

## v7.5.6 (2025-03-03)

### Feat

- implements a subscriptions-manager endpoint group to list guest roles

## v7.5.5 (2025-03-01)

### Fix

- turn webhooks paginated

## v7.5.4 (2025-02-26)

### Feat

- do implements the public tenant fetcher
- create a new option to allow authenticated users to interact with the mycelium downstream routes rithout registration

### Fix

- include the official image from mycelium-api docker hub in docker compose file

## v7.5.3 (2025-02-19)

### Refactor

- replace the head endpoint to check the user status by a get method with body response

## v7.5.2 (2025-02-19)

### Fix

- increase verbosity of the user creation process

### Refactor

- merge all scripts for database creation to a single up file

## v7.5.1 (2025-02-18)

### Fix

- include a desynchronozation element to avoid multiple synchronous execution of email and webhook dispatcher

### Refactor

- move the tracer to trace only during vault executions

## v7.5.0 (2025-02-16)

### Feat

- increase the ttl granularity of the cache for email and profile and the jwks response

### Fix

- update database model to be more migrationable
- prefix user tables to include the public schema name on create table schema
- fix the rpassword package name

### Refactor

- convert the response status to a ok status on verity the email registration status endpoint

## v7.4.0 (2025-02-10)

### Feat

- implements the userinfo cache

### Fix

- replace the cached crate by a native implementation of the caching functions

## v7.3.0 (2025-02-09)

### Feat

- include an extractor to check already the userinfo from the audience list

### Fix

- fix the email discovery process to include the user info collection from remote server

## v7.2.0 (2025-02-05)

### Feat

- **cached-profile**: finish the implementation for the profile caching
- wip - implements the key profile persistence to the redis database
- refactor the mycelium notifier to move the redis config init to a shared module

### Fix

- include secret resolver from myc-config in tracing
- **hashbrown**: hashbrown was identified as a high impact security issuer - upgrade version
- re-introduce the internal provider to the issuer fetcher flow
- upgrade the credencials checker to dinamically load identity providers
- remove log crate from the project

### Refactor

- split notifier models to a dedicated submodules and initialize the kv lib
- refactor project to inject notifier module instead instance along the api port
- refactor email fetcher middleware to turn it more verbose and dev friendly
- **fetch_profile_from_request**: split the fetch_profile_from_request to multiple submodules to turn the module arch as screamming
- fix english words

## v7.1.0 (2025-01-31)

### Feat

- wip - implements the async dispatcher functionality
- wip - do implements the asynchronous dispatching of webhooks
- review the weghook trigger names to improve the user understanding of their goal

### Fix

- remove guesting and revoke options from the webhook trigger
- fix the webhook async dispatch to avoid updates of the payload package and mirror important changes to database

### Refactor

- comment not implemented triggers options for the webhook
- move tracing and async dispatchers to dedicated modules

## v7.0.0 (2025-01-26)

### BREAKING CHANGE

- feature/gh29

### Feat

- migrate all prisma dependencies to diesel
- implements the licensed-resources diesel adapter
- implements the profile adapter for diesel
- implements the token adapter for diesel
- implements the users adapter for prisma
- implements methods for tenant tag for diesel adapter
- implements the webhook diesel adapters
- implements the tenant adapters for diesel
- implements the guest_user diesel adapter
- implemente diesel adapter to guest_role
- implements the error code diesel repository and initialize modules for other entities
- implements the account repository for diesel
- create the basis for the diesel adapter
- implements diesel models to mirror the sql implementation for the prisma adapter
- formalize the sql model as a sql manifest
- implements the account metadata crud
- improve the profile owner filtration and apply the improvement to the tenant owner endpoints
- implements the tenant ownership information into the profile

### Fix

- fix the meta endpoints for account meta management
- fix the guest to children account
- review the guest roles related operations
- **subscription-accounts**: fix the subscription accounts related operations
- wip - fix the guest roles diesel orm functionalities
- review the error code life cycle
- fix the webhook updating options to avoid updation of the url and triggers
- improve the tenant fetching and account fetching to adequate to the expected behaviour already validated with prisma
- fix the tenant operations related to the diesel database engnie
- fix the tenant fetching process
- fix the tenant fetching to migrato to native orm diesel query
- fix the totp lifecycle
- fix the user token invalidation on create a new one
- fix the profile fetching diesel query
- replace diesel uuid in models and repositories by string
- migrate the raw sql implementations injection of the fetch-profile-from-request to a native shaku module injection
- fix the diesel injection module to avoid modules implementation expositions
- fix the accout fetching adapter for prisma
- include the tenant-fetching repo to the tenant endpoints for tenant-owners
- include the meta field at the account model
- improve information about the account creation status on email checking response
- include the url option to the tenants-ownership field of the profile dto
- include tenant at the profile filtering
- **use-cases**: remove unused roles from profile filtering operation

### Refactor

- remove prisma client adapter
- move the account-tag to a dedicated module
- allow dead code for the profile fetching using telegram user use-case
- standardize the headers used to check an email status
- **profile**: split profile elements into independent modules
- rename use-cases to better inform users about functionality
- rename the standard account flag on licensed resources to a system account

### Perf

- include a slug to tenant-id index on tenant management model
- **account**: remove from serialization none fields of account dto
- **tenant**: remove from serialization none fields of tenant dto

## v6.6.0 (2025-01-07)

### Feat

- wip - review the guest system
- apply the new profile filtering validation
- **dto-profile**: upgrade profile to include a filtration state after each filter operation

### Fix

- **profile**: inform about deprecation of direct filter methods of profile
- fix the permissioning system
- inject the role name on guest to default account using service accounts
- **email-template**: move the email subject to the templates file allowing the internationalization by file

## v6.5.0 (2025-01-02)

### Feat

- turn the cert and key pem loading to use secret-resolver

### Fix

- fix the env variable collectino and migrate all auth variables to dynamically collected ones

## v6.4.0 (2025-01-01)

### Feat

- implements the secrets collection from vault

## v6.3.0 (2024-12-30)

### Feat

- **user-account-creation**: include a email notification to the new account creation workflow
- **gateway-manager/service**: implements the service listing for gateway managers
- expose the x-mycelium-request-id to the gateway user
- implements a new functionality to create all system roles by managers
- implements the injection of secrets through the gateway router
- implements the secrets service collection during the api gateway initialization
- wip - implements an aditional option to collect secrets from environment vault
- implements the gateway routes basic elements to check endpoints by api
- implements the invitation acceptance use cases and api

### Fix

- fix the webhook dispatching to decrypt secrets befhre send request to the incoming route
- set the utoipa redoc environment variable on the main file of the api port
- fix the parsing and verification of connection strings not working
- remove unused test
- review the licensed resources filtering from database
- include additional checks to allow routing to insecure downstream paths only if explicitly informed by users
- ensure the downstream service secrets to be removed from the gateway request
- fix the endpoints security definition

### Refactor

- remove the role submodule and move chindren modules to the root parent
- move the match forward address use-case to the gateway use-cases group
- rename the env-or-value functionality by secret-resolver
- refactor the route match use-case to use a correct base response from mycelium
- **secret-dto**: move the webhook secret dto to a independent dto named http-secret
- refactor the azure provider model to include new functionalities
- rename the standard folder to role-scoped in api prot
- move the match-forward-address to the api-port-router functionality
- rename the use-cases to mirror the application roles

## v6.2.0 (2024-12-01)

### Feat

- review the full api documentation and endpoints locations to improve the development experience and usability

### Fix

- replace the myc path url by adm

## v6.1.0 (2024-11-24)

### Feat

- implements the fetching the connection string from the request header
- upgrade the profile management to inject licensed resources as a url instead of a json object

### Fix

- fix the service endpoints to collect the tenant id from the connection string itself

### Refactor

- refactor the no-role guest endpoint to the new service route group
- refactor all routes to be more consistent
- replace the tenant id from the api route to use a x-mycelium-tenant-id header
- implements basic locale to the email template
- centralize the platform name and the platform url as the domain config instead to inject from the request url

### Perf

- improve the profile injection on internal roles to filter roles before send by downstream requests

## v6.0.0 (2024-11-13)

### BREAKING CHANGE

- feature/gh14

### Feat

- upgrade the azure authorization flow in replacement to the remote check
- implements the totp disable
- implements the two setp login using totp flow
- implements the totp activation
- implements the totp initial steps for the otp registration
- implements the secret encryption during the totl registration on user mfa flow
- implements the totp encryption and decryption
- implements the prisma and api injectors for token fetching module
- implements the token creation endpoint of guest-manager
- implements the noreply and internal emails naming
- implements the prisma adapter to create new connection string tokens and remove unused imports from native-errors in endpoints
- implements the connection string elements to generate service tokens
- implements the guest-to-children-account use-case as a api port endpoint
- implements the route filtration by permissioned roles
- implements the route level filtration by role
- implemens the children guest role management endpoints
- wip - implements the guest role children insertion and deletion features
- replace the smtp direct sender by a scheduler sender
- implements the tenant-manager endpoint related elements
- implements the tenant-owner endpoints
- implements the tenant management endpoints
- re-include the myc-cli crate on the project after fixing
- do implements the tenant management for prisma repositories
- wip - implements the prisma repository for tenant registration
- implements the abstract code for tenant tag management
- implement use-cases for metadata management in tenant-owner modules
- implement the account deletion repo
- do implement the tenant owner guest and revoke use-cases
- wip - do implement the tenant-owner functionalitie
- wip - do implement the new tenant based accounts management
- wip - do implement the base logic fot the tenant implementation
- implements the password recovery flow
- replace logs from the core use-cases by tracing
- implements the base for opentelemetry in lycelium
- improve the user activation code
- wip - initialize the migration from activation url to numeric token on create new user accounts
- improve the mycelium native auth to allow logins
- upgrade the internal authentication flow to generate simple authentication tokens from mycelium

### Fix

- finish the authorization code flow for the azure ad
- review the account list method to allow filter directly by account-type
- remove url query string from webhook response to avoid expose secrets
- fix the guest to default subscription account name inserted at the account name
- adjust the webhook dispatching method based on the webhook trigger
- fix the webhook lifecycle to live as a more verbose to the final users
- include the redaction function on get webhook from database
- remove greather than equal from licensed resource fetching using prisma to use only equal to permissions
- replace the myc00013 response from profile permissions check by a myc00019 code
- allow users and staffs to use role protected routes
- fix the email processing counter and fix the child role invitation use-case to avoid guest to different roles that the target one
- fix staff endpoints to upgrade and downgrade accounts
- replace the profile injection to responds with unauthorized instead of forbidden
- fix the guest process
- replace the guesting email template element to use the tera template
- fix the email consumption queue processor
- remove google and azure endpoints from logging ignore rules
- reintroduce the system and users management endpoints
- re-introduce the staff endpoints
- fix account and licensed resources prisma repositories
- fix database connection between tenant and account
- migrate sql database before fix the prisma repositories
- rename revoke tenant owner wrongly named use-case function
- fix role-related account propagation through the webhook element
- solve the permission check for the tenant operations to include validation over the manager and staff accounts
- fix return tyoe of login function
- move the tracing initialization to move it to the root of the main api function
- include expiration time during the account activation token fetching from database
- move the email template location and fix the email verification code generation

### Refactor

- refactor providers to standardize modules
- move the shared functions from the roles module to a support module
- refactor webhooks to follow de main stream format widely used in web applications
- rename the token generator for the account associated connection string use-case
- move the email sender to a dedicated module shared between use-cases and create a new mapped-error-to-http-response mapping handled
- rename user to users and subscription by subscriptions as default actors and mirror to dependent elements
- refactor the permissions to be a integer with read write and read-write options only
- replace all crud related permissioner elements by simple read-write ones
- rename child insertion and removal use-cases files
- move the guest-role to a dedicated dto module
- refactor the mycelium smtp to be a general purpose notifier
- rename all json-error occurrent by http-json-error crate
- move otel acessory functions from main api port file to a dedicated otel file
- refactor endpoints to use the standard error code wrappers
- turn tenant endpoints of manager api module a file
- reactivate guests endpoints and move it to the subscription-manager endpoints group
- rename the checked status to verified or unverified in verbose-status
- adequate all core elements to include tenant on the profile filtering
- move the guest to subscription-manager instead to guest-managers
- move the staff tenant management elements to manager management elements
- rename the user and subscription manager roles
- move the session_token to account-life-cycle module

## v5.0.8 (2024-04-25)

### Fix

- wip - improve the google authentication checking logs and the api port logs to allow better debug

## v5.0.7 (2024-04-11)

## v5.0.6 (2024-04-09)

## v5.0.5 (2024-04-09)

## v5.0.4 (2024-04-09)

### Fix

- inplements the google checks for oauth2 token online

## v5.0.3 (2024-04-07)

### Fix

- **account-and-users**: turn the default user and account creation process to considering email as case-insensitive

## v5.0.2 (2024-03-21)

### Fix

- fix actix-web corst to return specifig headers into responses

## v5.0.1 (2024-03-11)

### Fix

- rename gateway request estractors of the injected profile

## v5.0.0 (2024-03-08)

### BREAKING CHANGE

- main

### Perf

- **licensed-resources**: replace the licensed resources fetching to use a view instead to perform multiple joins to fetch licenses contents

## v4.16.0 (2024-02-26)

### Feat

- **generic-maps**: implements a generic map to allow ynamic typing of hashmaps
- replace error factories to accept generic types as the error message argument

## v4.15.3 (2024-02-22)

### Refactor

- move the sql adapters used during the profile extraction from requests of the api middleware to the own funciton that execute the action
- move the mycelium-http-tools to a dedicated module and kept the api related middleware elements to the api port module
- move the api to a backward directory given the absence of the base myc-http-tools library

## v4.15.2 (2024-02-21)

### Refactor

- move the myc-http-tools to a dedicated package

## v4.15.1 (2024-02-15)

### Fix

- orient all internal paths dependencies to the project path instead to use directly into the workspace
- implements related accounts filtering default methods

## v4.15.0 (2024-02-15)

### Feat

- implements a reelated account enumerator that allow users to check permissions for a specified account or itself has privileged permissions

## v4.14.0 (2024-02-01)

### Feat

- upgrade default festures to include new error handlings

### Fix

- fix the subscription accounts search to include tag search and a case insensitive search including uuid targeted search

## v4.13.0 (2024-01-29)

### Feat

- implements the tags creation endpoint entities and use-cases
- implements the tag models into domain dtos and adapters

## v4.12.0 (2024-01-29)

### Feat

- implements the slug name to allow accounts renaming without rename

## v4.11.1 (2024-01-23)

### Fix

- **subscription-account-propagation**: fix the subscription account propagation use-cases

## v4.11.0 (2024-01-23)

### Feat

- include default actors as an public object into the myc-http-tools

## v4.10.0 (2024-01-23)

### Feat

- upgrade webhook propagation functionalities to passthrough the bearer token together request

## v4.9.1 (2024-01-22)

### Fix

- **subscription-account**: include the subscription-accounts name and flags updating endpoint
- fix tokio dependencies of config and adapters and update dockerfile to includ base into installatin

## v4.9.0 (2024-01-21)

### Feat

- implements the new base package to replace the clean-base package
- **subscription-account**: implements a use-case and endpoint to update accounts name and flags

## v4.8.3 (2024-01-21)

### Fix

- fix the subscription account creation elements to allow propagation of default-users accounts

## v4.8.2 (2024-01-21)

### Fix

- upgrade all std use-cases to responds to default account on check actinos permissions from profile

## v4.8.1 (2024-01-20)

### Fix

- upgrade the prisma model to include the flag is-default into database and update associated adapters

## v4.8.0 (2024-01-19)

### Feat

- implements the api port to interact with the propagation use-case

## v4.7.16 (2024-01-17)

### Fix

- **router**: fix router to correctely check for all and none methods

## v4.7.15 (2024-01-17)

### Fix

- **router**: fix http method filtragion from gateway router

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
