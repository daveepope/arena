-- instrument_reading_db_schema.sql
-- Smell-O-Scope / Smellometer demo schema (Futurama)

begin;

create schema if not exists instrument_reading;
set search_path to instrument_reading;

-- instrument (id, name, description)
create table if not exists instrument (
  id bigserial primary key,
  name text not null unique,
  description text
);

-- user (id, name)
create table if not exists "user" (
  id bigserial primary key,
  name text not null unique
);

-- reading (id, userId, value int, comment)
create table if not exists reading (
  id bigserial primary key,
  "userId" bigint not null references "user"(id) on delete cascade,
  value int not null,
  comment text
);

-- Seed: just the Smell-O-Scope for now
insert into instrument (name, description) values
(
  'Smell-O-Scope',
  'A device that allows people to smell distant cosmic objects.'
)
on conflict (name) do nothing;

-- Seed users
insert into "user" (name) values
  ('Professor Farnsworth'),
  ('Philip J. Fry'),
  ('Turanga Leela'),
  ('Bender Bending Rodríguez')
on conflict (name) do nothing;

-- Seed readings (value = made-up "smell units"; comment = character quotes/jokes)
insert into reading ("userId", value, comment)
select u.id, r.value, r.comment
from "user" u
join (values
  ('Philip J. Fry', 10, 'Jupiter... smells like strawberries.'),
  ('Philip J. Fry', 100, 'Bllllalarghhhghg...'),
  ('Professor Farnsworth', 100, 'Ohhh jeez oh man! A stench so foul its right off the funk-o-meter!'),
  ('Turanga Leela', 100, 'I think its moving?'),
  ('Professor Farnsworth', 100, 'You may have discovered the smelliest thing in the known universe!'),
  ('Bender Bending Rodríguez', 100, 'Oh oh name it after me!')
) as r(user_name, value, comment)
  on r.user_name = u.name;

commit;