CREATE TABLE users (
    id         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    email      VARCHAR(255) NOT NULL UNIQUE, -- la contrainte UNIQUE crée l'index utilisé au login
    password   VARCHAR(255) NOT NULL,        -- hash bcrypt
    first_name VARCHAR(100) NOT NULL,
    last_name  VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Met à jour updated_at à chaque UPDATE (fonction créée par la migration initiale de Diesel)
SELECT diesel_manage_updated_at('users');
