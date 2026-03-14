CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE);
CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), total REAL, amount REAL, created_at TEXT DEFAULT CURRENT_TIMESTAMP);
CREATE INDEX idx_orders_user ON orders(user_id);
