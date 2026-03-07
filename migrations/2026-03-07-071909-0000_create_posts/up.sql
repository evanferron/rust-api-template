-- Your SQL goes here

CREATE TABLE IF NOT EXISTS posts (
    id         UUID         PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title      VARCHAR(255) NOT NULL,
    content    TEXT         NOT NULL,
    published  BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP    NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP    NOT NULL DEFAULT NOW()
);

-- Index sur user_id pour les lookups "posts d'un user"
CREATE INDEX IF NOT EXISTS idx_posts_user_id ON posts(user_id);

-- Réutilise le trigger updated_at déjà créé dans la migration users
CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON posts
    FOR EACH ROW
EXECUTE FUNCTION trigger_set_updated_at();