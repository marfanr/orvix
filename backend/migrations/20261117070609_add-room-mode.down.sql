-- Add down migration script here

ALTER TABLE room
DROP COLUMN IF EXISTS mode;

DROP TYPE IF EXISTS ROOM_MODE;