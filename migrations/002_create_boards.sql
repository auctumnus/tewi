create table board_categories (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  name VARCHAR NOT NULL
);

create table boards (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  slug VARCHAR NOT NULL,
  name VARCHAR NOT NULL,
  description TEXT NOT NULL,
  category_id UUID REFERENCES board_categories(id)
);