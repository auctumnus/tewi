create table refs (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    from_post_id uuid not null references posts(id) on delete cascade,
    to_post_id uuid not null references posts(id) on delete cascade
);

create index idx_refs_from_post_id ON refs(from_post_id);
create index idx_refs_to_post_id ON refs(to_post_id);