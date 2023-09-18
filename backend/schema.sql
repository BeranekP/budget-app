-- Create budget item table if it does not exist
CREATE TABLE IF NOT EXISTS budget_items (
    id serial PRIMARY KEY,
    type_id INT,
    amount NUMERIC,
    category_id INT,
    CONSTRAINT item_type FOREIGN KEY(type_id) REFERENCES item_types(type_id),
    name VARCHAR(255),
    description TEXT,
    CONSTRAINT category FOREIGN KEY(category_id) REFERENCES category(category_id),
    date TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);


