CREATE TABLE faucet_http_events (
    namespace TEXT,
    target TEXT,
    ip_addr INET,
    method TEXT,
    path TEXT,
    version TEXT,
    status SMALLINT,
    user_agent TEXT,
    elapsed BIGINT,
    time TIMESTAMPTZ
);

