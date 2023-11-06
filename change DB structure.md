# Change DB structure

I probably should change the whole DB structure to something like that:

## products:

| id | model | quantity |
|----|-------|----------|
| 1  | M1N2  | 2        |
| 2  | M1E4  | 1        |
| 3  | M3E6  | 4        |

```SQL
CREATE TABLE products (
id SERIAL PRIMARY KEY,
model VARCHAR(10),
quantity INT
);

INSERT INTO products (id, model, quantity) 
VALUES (1, 'M1N2', 2), (2, 'M1E4', 1), (3, 'M3E6', 4);

INSERT INTO products (id, model, quantity) 
VALUES (1, 'M1N2', 1) 
ON CONFLICT(id) DO UPDATE SET quantity = quantity + 1;
```

## serial_numbers:

| id | model_id | serial_number |
|----|----------|---------------|
| 1  | 1        | M1001         |
| 2  | 1        | M1002         |
| 3  | 2        | M1003         |
| 4  | 5        | M3001         |
| 5  | 5        | M3002         |
| 6  | 5        | M3003         |
| 7  | 6        | M3004         |
| 8  | 6        | M3005         |
| 9  | 6        | M3006         |

```SQL
CREATE TABLE serial_numbers (
id SERIAL PRIMARY KEY,
model_id INT REFERENCES products(id),
serial_number VARCHAR(5) UNIQUE
);

INSERT INTO serial_numbers (id, model_id, serial_number) VALUES
(1, 'M1001'), (1, 'M1002'), (2, 'M1003'),
(5, 'M1004'), (5, 'M3001'), (5, 'M3002'),
(6, 'M3003'), (6, 'M3004');
```

## sold:

| id | model_id | serial_number | sale_date           | customer_id |
| 1  | 1        | M1001         | 2023-09-06 11:09:22 | Kurmanjan
| 2  | 4        | M2003         | 2023-10-13 04:24:38 | Mary

```SQL
CREATE TABLE IF NOT EXISTS sold (
id SERIAL PRIMARY KEY,
model_id INT REFERENCES products(id),
serial_number VARCHAR(5) UNIQUE,
sale_date DATE,
customer_id VARCHAR(20)
);

INSERT INTO sold (id, model_id, serial_number, sale_date, customer_id) VALUES
(1, 'M1001', '2023-09-06 11:09:22', 'Kurmanjan'),
(4, 'M2003', '2023-10-13 04:24:38', 'Mary');
```

Move from products to sold example:
```SQL
WITH moved AS (SELECT * FROM products WHERE id = 2) 
INSERT INTO sold (id, model, quantity, date) 
SELECT id, model, quantity, datetime('now') FROM moved; 
DELETE FROM products WHERE id = 2;
```
