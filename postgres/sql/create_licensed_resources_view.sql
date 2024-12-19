
CREATE OR REPLACE VIEW licensed_resources AS
SELECT DISTINCT
	ac.id AS acc_id,
	ac.name AS acc_name,
	ac.is_default AS is_acc_std,
	gr.id AS gr_id,
	gr.name AS gr_name,
	gr.permission AS gr_perm,
	rl.name AS rl_name,
    gu.email AS gu_email,
	gu.was_verified AS gu_verified,
	ac.tenant_id AS tenant_id
FROM
	guest_user_on_account AS ga
JOIN
	guest_user AS gu
ON
	ga.guest_user_id = gu.id 
JOIN
	guest_role AS gr
ON
	gr.id = gu.guest_role_id
JOIN
	account AS ac
ON
	ac.id = ga.account_id
JOIN
	role AS rl
ON
	rl.id = gr.role_id
ORDER BY
    gu_email, rl_name, acc_id, gr_id;
