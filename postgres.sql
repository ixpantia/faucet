CREATE TABLE faucet_http_events (
    request_uuid UUID,
    namespace TEXT,
    target TEXT,
    worker_route TEXT,
    worker_id INT,
    ip_addr INET,
    method TEXT,
    path TEXT,
    query_params TEXT,
    version TEXT,
    status SMALLINT,
    user_agent TEXT,
    elapsed BIGINT,
    time TIMESTAMPTZ
);

CREATE INDEX faucet_http_events_request_uuid_idx 
ON faucet_http_events USING BTREE (request_uuid);

