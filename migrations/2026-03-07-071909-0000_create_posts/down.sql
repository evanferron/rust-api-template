-- This file should undo anything in `up.sql`
DROP TRIGGER IF EXISTS set_updated_at ON posts;
DROP TABLE IF EXISTS posts;