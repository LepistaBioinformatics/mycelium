-- Create all databases

SELECT 'CREATE DATABASE "mycelium"'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'mycelium');\gexec

-- Create user roles

CREATE ROLE "myc-service-role";

-- Assign permissions to the role

GRANT CONNECT ON DATABASE mycelium TO "myc-service-role";
GRANT USAGE ON SCHEMA public TO "myc-service-role";
GRANT ALL ON ALL TABLES IN SCHEMA public TO "myc-service-role";

-- Create user

CREATE USER "myc-user" WITH PASSWORD 'myc-password';

-- Include user in role

GRANT "myc-service-role" TO "myc-user";
