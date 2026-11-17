-- SPDX-FileCopyrightText: 2026 Mohammad Arfan
--
-- SPDX-License-Identifier: AGPL-3.0

CREATE TABLE IF NOT EXISTS users (
    id       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username varchar(100) NOT NULL UNIQUE,
    email    varchar(254) NOT NULL UNIQUE,
    password text NOT NULL,
    salt     text
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    jti               uuid PRIMARY KEY,
    user_id           uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    family_id         uuid NOT NULL,
    used_at           timestamptz,
    revoked_at        timestamptz,
    expires_at        timestamptz NOT NULL,
    family_expires_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS refresh_tokens_user_id_idx
    ON refresh_tokens (user_id);

CREATE INDEX IF NOT EXISTS refresh_tokens_family_id_idx
    ON refresh_tokens (family_id);

CREATE TABLE IF NOT EXISTS room (
    id       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    alias    text NOT NULL UNIQUE,
    passcode text,
    title    text NOT NULL DEFAULT 'Untitled Meeting',
    options  jsonb NOT NULL DEFAULT '{}'::jsonb,
    start_at timestamptz
);

CREATE INDEX IF NOT EXISTS room_owner_id_idx
    ON room (owner_id);
