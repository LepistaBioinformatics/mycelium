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


## default_users
[ x ] default_users/account/create_default_account.rs
[   ] default_users/account/update_own_account_name.rs

## managers
[   ] managers/account/create_subscription_account.rs
[   ] managers/guest/guest_user.rs
[   ] managers/guest_role/create_guest_role.rs
[   ] managers/guest_role/delete_guest_role.rs
[   ] managers/guest_role/update_guest_role_name_and_description.rs
[   ] managers/guest_role/update_guest_role_permissions.rs
[   ] managers/guest/uninvite_guest.rs
[   ] managers/role/create_role.rs
[   ] managers/role/delete_role.rs
[   ] managers/role/update_role_name_and_description.rs

## service
[   ] service/fetch_profile_from_email.rs

## shared
[   ] shared/account/approve_account.rs
[   ] shared/account/change_account_activation_status.rs
[   ] shared/account/downgrade_account_status.rs
[   ] shared/account_type/get_or_create_default_account_types.rs
[   ] shared/account/upgrade_account_privileges.rs

## staff
[   ] staff/create_seed_staff_account.rs
