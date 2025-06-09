# Load Balancing Strategies in Faucet

Load balancing is a critical component for distributing network traffic across multiple Faucet worker processes, each typically running an instance of an R application (like Shiny, Plumber, or Quarto Shiny). This distribution ensures that no single worker becomes overwhelmed, leading to improved responsiveness, availability, and reliability of your deployed applications. This document outlines the load balancing strategies available in Faucet, their use cases, and their respective advantages and disadvantages.

Faucet allows you to configure the load balancing strategy using the `--strategy` command-line option or the `FAUCET_STRATEGY` environment variable. The available strategies are:

*   Round Robin
*   IP Hash
*   Cookie Hash

## Default Strategies

Faucet applies default strategies based on the application type if not explicitly specified:
*   **Shiny and Quarto Shiny applications:** Default to `ip-hash` to ensure session persistence.
*   **Plumber APIs:** Default to `round-robin` as they are often stateless.

## Common Features: Worker Health and Retries

All load balancing strategies in Faucet incorporate a mechanism to handle offline worker processes. If a selected backend worker is detected as offline:

1.  Faucet will log the attempt to connect to the offline worker.
2.  An exponential backoff retry mechanism is employed.
3.  **Behavior upon worker failure differs by strategy:**
    *   **Round Robin:** After a short wait (`WAIT_TIME_UNTIL_RETRY`), Faucet will attempt to route the request to the *next available* worker in the sequence.
    *   **IP Hash & Cookie Hash:** Faucet will continue to retry connecting to the *originally designated* worker. This means requests for that specific worker are effectively "held" and will experience latency or appear to hang until the worker is back online or the request times out. Clients are not automatically rerouted to a different worker because that would break session persistence.

## 1. Round Robin

### Description
The Round Robin strategy distributes incoming requests to Faucet's worker processes in a sequential order. Each new request is sent to the next worker in the list. When the end of the list is reached, the load balancer returns to the beginning and starts over.

### Use Cases
*   **Stateless Applications:** Ideal for stateless applications like many Plumber APIs, where each request can be handled independently by any worker.
*   **Simple Deployments:** Suitable when all worker processes are expected to have similar processing capabilities.

### Pros
*   **Simplicity:** Easy to understand and implement.
*   **Even Distribution (ideal conditions):** If all workers are healthy and have similar capacities, Round Robin can distribute traffic relatively evenly.
*   **Low Overhead:** Minimal computational cost for the load balancer.
*   **Resilience to Worker Failure:** If a worker goes offline, requests are automatically routed to the next available worker after a brief delay.

### Cons
*   **Ignores Worker Load:** Does not take into account the current load on individual worker processes (beyond basic online/offline checks).
*   **No Session Persistence:** Clients might be directed to different workers on subsequent requests. This makes it **unsuitable** for stateful applications like Shiny or Quarto Shiny apps that require session stickiness (e.g., maintaining user-specific data or input states).
*   **Uneven Distribution with Varying Capacities:** If worker processes have different underlying capacities (though Faucet typically spawns identical R processes), some might become overloaded.

## 2. IP Hash

### Description
The IP Hash strategy uses the client's IP address to determine which Faucet worker process will handle the request. A hash function is applied to the client's IP address, and the resulting hash value consistently maps to a specific worker.

**Important for Reverse Proxy Setups:** If Faucet is running behind a reverse proxy (e.g., Nginx, Apache), it's crucial to configure the `--ip-from` option (or `FAUCET_IP_FROM` environment variable) correctly. This tells Faucet whether to use the direct client IP or an IP from a header like `X-Forwarded-For` or `X-Real-IP`, ensuring accurate IP identification for this strategy.

### Use Cases
*   **Stateful Applications (Default for Shiny/Quarto):** Essential for applications like Shiny and Quarto Shiny apps that require session persistence. It ensures that a client is consistently routed to the same worker process, maintaining their session state.
*   **Caching Benefits:** Can improve cache hit rates on the worker if data is cached based on user interactions.

### Pros
*   **Session Persistence:** Guarantees that requests from the same client IP are consistently routed to the same worker, crucial for stateful R applications.
*   **Deterministic Routing:** The same IP will always route to the same worker (assuming the pool of workers hasn't changed).

### Cons
*   **Uneven Load Distribution:**
    *   If a few IP addresses generate a disproportionately large volume of traffic, the workers assigned to those IPs can become overloaded.
    *   Clients behind a Network Address Translation (NAT) gateway or a large corporate proxy will all appear to have the same source IP. All these clients will be directed to the same worker, potentially overwhelming it.
*   **Changing Client IPs:** Session persistence can be lost if a client's IP address changes during their session (e.g., mobile users switching between Wi-Fi and cellular data).
*   **Worker Failures:** If a designated worker goes down, requests for clients hashing to that worker will be held and retried against the *same* worker, leading to delays for those users until the worker is restored. They are not automatically rerouted to preserve session integrity.

## 3. Cookie Hash

### Description
The Cookie Hash strategy achieves session persistence by using an HTTP cookie named `FAUCET_LB_COOKIE`. When a request arrives:
1.  Faucet checks for the `FAUCET_LB_COOKIE`.
2.  If the cookie exists and contains a valid UUID, Faucet uses this UUID to consistently select a backend worker process.
3.  If the cookie is not present, is invalid, or the strategy is `CookieHash` and no suitable cookie UUID is found, **Faucet generates a new UUID**.
4.  This UUID (either extracted or newly generated) is then used to determine the worker.
5.  Crucially, Faucet will **set (or update) the `FAUCET_LB_COOKIE` in the HTTP response**, including the UUID. This ensures that subsequent requests from the same client browser will include this cookie, directing them to the same worker.

This mechanism ensures the client is consistently directed to the same worker for subsequent requests as long as their browser accepts and sends cookies.

### Use Cases
*   **Robust Stateful Applications:** Provides reliable session persistence for Shiny, Quarto Shiny, or other stateful applications. It's particularly beneficial when client IP addresses are not stable or when many clients might share an IP address (e.g., users behind large NATs or proxies).
*   **Fine-grained Session Control:** Offers more precise control over session stickiness than IP Hash, as it relies on a unique identifier (the cookie's UUID) specific to the client's session, managed by Faucet.

### Pros
*   **Reliable Session Persistence:** More robust than IP Hash in scenarios with dynamic client IPs or NAT, as it depends on the Faucet-managed cookie.
*   **Better Load Distribution (than IP Hash in NAT scenarios):** Can distribute load more evenly than IP Hash when many users share the same source IP, as each user's browser session will get its own `FAUCET_LB_COOKIE` with a unique UUID.
*   **Deterministic Routing:** The same cookie UUID will consistently route to the same worker (assuming the worker pool is stable).
*   **Automatic Cookie Management by Faucet:** Faucet handles the generation and setting of the necessary cookie, simplifying setup.

### Cons
*   **Client Cookie Support:** Relies on clients accepting and sending cookies. If a client has cookies disabled, this strategy will not provide session persistence.
*   **Cookie Overhead:** Involves the standard overhead of HTTP cookie transmission and processing, though Faucet's management is efficient.
*   **Worker Failures:** Similar to IP Hash, if a worker designated by a cookie hash goes down, requests associated with that cookie hash will be held and retried against the *same* worker, potentially causing delays for affected users.
*   **Initial Simultaneous Requests:** As noted in the Faucet source code, if a browser sends multiple *simultaneous* initial requests before the first `Set-Cookie` response is processed and returned by the browser, those initial requests might briefly hit different workers before settling on the one determined by the eventually set cookie. This is a minor edge case for most applications.

---

Choosing the right load balancing strategy in Faucet depends heavily on the specific requirements of your R application, particularly its statefulness, and your deployment environment (e.g., standalone vs. behind a reverse proxy). For Shiny and Quarto Shiny apps, `ip-hash` (default) or `cookie-hash` are generally recommended. For stateless Plumber APIs, `round-robin` (default) is often sufficient.