-- Create all databases

SELECT 'CREATE DATABASE "mycelium"'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'mycelium');\gexec
