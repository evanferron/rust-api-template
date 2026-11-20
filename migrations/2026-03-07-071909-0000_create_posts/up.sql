CREATE TABLE posts (
    id         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title      VARCHAR(255) NOT NULL,
    content    TEXT         NOT NULL,
    published  BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Posts d'un utilisateur, du plus récent au plus ancien (pagination) ; sert aussi d'index pour la clé étrangère
CREATE INDEX idx_posts_user_id_created_at ON posts (user_id, created_at DESC);

-- Met à jour updated_at à chaque UPDATE (fonction créée par la migration initiale de Diesel)
SELECT diesel_manage_updated_at('posts');
