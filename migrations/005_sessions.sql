create table sessions (
    id serial primary key,
    admin_id uuid not null references admins(id),
    token text not null,
    created_at TIMESTAMPTZ not null default now(),
    expires_at TIMESTAMPTZ not null
);

create index idx_sessions_session_token ON sessions(token);