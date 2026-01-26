create table attachment_policies (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    board_id UUID NOT NULL REFERENCES boards(id),
    mime_types VARCHAR[] NOT NULL,
    size_limit INT NOT NULL, -- in bytes
    enable_spoilers boolean NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);