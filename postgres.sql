CREATE TABLE faucet_http_events (
    request_uuid UUID,
    namespace TEXT,
    version TEXT,
    target TEXT,
    worker_route TEXT,
    worker_id INT,
    ip_addr INET,
    method TEXT,
    path TEXT,
    query_params TEXT,
    http_version TEXT,
    status SMALLINT,
    user_agent TEXT,
    elapsed BIGINT,
    time TIMESTAMPTZ NOT NULL
);

-- For use in timescale
-- SELECT create_hypertable('faucet_http_events', by_range('time'));


CREATE TABLE faucet_log_events (
    namespace TEXT,
    version TEXT,
    target TEXT,
    time TIMESTAMPTZ NOT NULL,
    event_id UUID,
    parent_event_id UUID,
    level TEXT,
    event_type TEXT,
    message TEXT,
    body JSONB
);
