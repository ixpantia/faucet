# Telemetry in Faucet

Faucet includes a telemetry feature designed to help you monitor the performance and usage patterns of your deployed applications. When enabled, Faucet can send telemetry data to a PostgreSQL database, allowing for analysis and insights into how your Faucet instances and the underlying R applications are operating.

This document outlines how to configure and utilize Faucet's telemetry capabilities.

## Database Setup

Before enabling telemetry, you need to set up your PostgreSQL database with the required table. Faucet will send its telemetry data to a table named `faucet_http_events`.

You can create this table using the following SQL command:

```sql
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
```

**Note for TimescaleDB Users:**

If you are using TimescaleDB, you can optionally convert this table into a hypertable for better time-series data management. After creating the table as shown above, you can run the following SQL command:

```sql
SELECT create_hypertable('faucet_http_events', by_range('time'));
```
This step is specific to TimescaleDB and enhances its capabilities for handling large volumes of time-series data.

## Enabling and Configuring Telemetry

Telemetry in Faucet is disabled by default. To enable it, you must provide a PostgreSQL connection string. Configuration can be done via command-line options or environment variables.

### Key Configuration Options:

1.  **PostgreSQL Connection String:**
    *   **CLI:** `--telemetry-postgres-string <CONNECTION_STRING>`
    *   **Environment Variable:** `FAUCET_TELEMETRY_POSTGRES_STRING=<CONNECTION_STRING>`
    *   **Description:** This is the essential setting to enable telemetry. The connection string should be in a format suitable for connecting to your PostgreSQL database (e.g., `postgresql://user:password@host:port/database`). Faucet will use this to send telemetry data.
    *   **Default:** `None` (Telemetry disabled)

2.  **Telemetry Namespace:**
    *   **CLI:** `--telemetry-namespace <NAMESPACE>`
    *   **Environment Variable:** `FAUCET_TELEMETRY_NAMESPACE=<NAMESPACE>`
    *   **Description:** Allows you to define a namespace for the telemetry data. This is useful if you are collecting data from multiple Faucet instances or different services into the same database, helping to segment and identify the source of the data.
    *   **Default:** `faucet`

3.  **Telemetry Version:**
    *   **CLI:** `--telemetry-version <VERSION>`
    *   **Environment Variable:** `FAUCET_TELEMETRY_VERSION=<VERSION>`
    *   **Description:** Specifies the version of the service or application being run/monitored by Faucet. This can be your application's version or Faucet's version itself. It's helpful for filtering telemetry data and correlating observations with specific deployments.
    *   **Default:** `None`

For more details on these options, refer to the [Command-Line Options](./options.md) page.

## Data Collected

Faucet's telemetry system is designed to capture information relevant to the operational aspects of the server and the applications it manages. While the exact schema and data points may evolve, the general categories of data collected include:

*   **Request/Response Metrics:** Information about incoming HTTP requests and the responses generated, such as request paths, response status codes, and latencies.
*   **Worker Performance:** Data related to the behavior of individual worker processes, potentially including processing times and error rates.
*   **Load Balancing Events:** Information about how requests are distributed if load balancing strategies are in use.
*   **Instance Information:** Details such as the configured namespace and version, to help contextualize the data.

The data is structured to be stored in a PostgreSQL database, allowing for SQL-based querying and integration with various analytics and visualization tools.

## Utilizing Telemetry Data

Once telemetry is configured and Faucet is sending data to your PostgreSQL database, you can:

*   **Monitor Application Health:** Track error rates, response times, and other key performance indicators (KPIs) to ensure your applications are running smoothly.
*   **Understand Usage Patterns:** Analyze request volumes, popular endpoints, and user activity to gain insights into how your applications are being used.
*   **Troubleshoot Issues:** Correlate telemetry data with logs and other monitoring tools to diagnose and resolve problems more effectively.
*   **Capacity Planning:** Observe resource utilization and performance trends over time to make informed decisions about scaling your infrastructure.
*   **Performance Optimization:** Identify bottlenecks or slow operations by examining request latencies and worker performance data.

You can connect to the PostgreSQL database using standard SQL clients, business intelligence tools, or custom scripts to query and visualize the collected telemetry data according to your needs.