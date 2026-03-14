CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT, email TEXT, active INT);
CREATE TABLE orders(id INTEGER PRIMARY KEY, customer_id INT, total REAL);
