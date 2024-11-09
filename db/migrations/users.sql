CREATE TABLE
    IF NOT EXISTS users (
        id BIGINT PRIMARY KEY, -- Discord User Id
        permission SMALLINT NOT NULL -- Permissions within the App
    );