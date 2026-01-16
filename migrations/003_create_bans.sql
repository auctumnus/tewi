create table ips (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    ip_address INET NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

create table bans (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    ip_id UUID REFERENCES ips(id) ON DELETE CASCADE,
    reason VARCHAR NOT NULL,
    banned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    banned_by UUID REFERENCES admins(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ
);