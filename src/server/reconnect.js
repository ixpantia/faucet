function createPingFrame(data) {
  const buffer = new Uint8Array(data.length + 1);
  buffer[0] = 0x8A; // Opcode for Ping
  for (let i = 0; i < data.length; i++) {
    buffer[i + 1] = data.charCodeAt(i); // Convert string to char codes
  }
  return buffer;
}

/**
 * A WebSocket wrapper that automatically reconnects and maintains a session ID.
 * The session ID is generated on instantiation and added as a query parameter
 * to the WebSocket URL, allowing the server to re-associate the connection.
 */
class ReconnectingWebSocket {
  /**
   * @param {string} url The URL to connect to.
   * @param {string|string[]} [protocols] The protocols to use.
   * @param {object} [options] Configuration options.
   * @param {number} [options.maxReconnectAttempts=5] Maximum number of reconnect attempts.
   * @param {number} [options.reconnectDelay=2000] Delay in ms between reconnect attempts.
   * @param {string} [options.sessionQueryParam='sessionId'] The name of the query param for the session ID.
   * @param {number} [options.pingInterval=1000] Delay in ms for sending ping messages.
   * @param {number} [options.pongTimeout=2000] Delay in ms to wait for a pong before closing.
   */
  constructor(url, protocols, options = {}) {
    // --- Public Interface ---
    this.onopen = null;
    this.onclose = null;
    this.onmessage = null;
    this.onerror = null;
    this.onreconnect = (details) => {
      var reconnecting_el = document.getElementById("faucet-reconnecting-msg");
      if (!reconnecting_el) {
        const el = document.createElement("div");

        // Style the element to be a floating notification
        el.style.position = "fixed";
        el.style.bottom = "0";
        el.style.left = "5px";
        el.style.padding = "5px";
        el.style.backgroundColor = "rgba(220, 220, 220, 0.4)";
        el.style.color = "black";
        el.style.borderRadius = "5px 5px 0 0";
        el.style.zIndex = "10001";
        el.style.fontFamily = "sans-serif";

        el.id = "faucet-reconnecting-msg";
        el.textContent = "Reconnecting...";

        document.body.appendChild(el);
      }
    };

    this.onreconnected = () => {
      var reconnecting_el = document.getElementById("faucet-reconnecting-msg");
      if (reconnecting_el) {
        reconnecting_el.remove();
      }
    };

    // --- Internal State ---
    this._protocols = protocols;
    this._ws = null;
    this._reconnectAttempts = 0;
    this._totalReconnectAttempts = 0;
    this._forcedClose = false;
    this._pingIntervalId = null;
    this._pongTimeoutId = null;

    if (window.crypto && typeof window.crypto.randomUUID === "function") {
      this._sessionId = window.crypto.randomUUID();
    } else {
      // Basic fallback for older browsers.
      this._sessionId = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(
        /[xy]/g,
        function (c) {
          const r = (Math.random() * 16) | 0;
          const v = c === "x" ? r : (r & 0x3) | 0x8;
          return v.toString(16);
        },
      );
    }

    const sessionQueryParam =
      options.sessionQueryParam != null
        ? options.sessionQueryParam
        : "sessionId";
    const separator = url.includes("?") ? "&" : "?";
    this._url = `${url}${separator}${sessionQueryParam}=${this._sessionId}`;

    this._maxReconnectAttempts =
      options.maxReconnectAttempts != null ? options.maxReconnectAttempts : 50;
    this._reconnectDelay =
      options.reconnectDelay != null ? options.reconnectDelay : 500;
    this._maxReconnectTime =
      options.maxReconnectTime != null ? options.maxReconnectTime : 10000; // 10 seconds
    this._lastDisconnectTime = null;

    // Heartbeat options
    this._pingIntervalDuration =
      options.pingInterval != null ? options.pingInterval : 1000;
    this._pongTimeoutDuration =
      options.pongTimeout != null ? options.pongTimeout : 2000;

    // Initial connection
    this.connect();
  }

  /**
   * Initiates the WebSocket connection.
   */
  connect() {
    console.log(`ReconnectingWebSocket: Connecting to ${this._url}...`);
    this._ws = new WebSocket(
      `${this._url}&attempt=${this._totalReconnectAttempts}`,
      this._protocols,
    );

    this._ws.onopen = (event) => {
      console.log(
        `ReconnectingWebSocket: Connection opened with Session ID: ${this._sessionId}`,
      );
      this._startPinging(); // Start heartbeat
      if (this.onopen) {
        this.onreconnected();
        this.onopen(event);
      }
    };

    this._ws.onmessage = (event) => {
      // Check for pong message from server heartbeat
      if (event.data === "pong") {
        clearTimeout(this._pongTimeoutId);
        return; // Don't forward pong messages to the user's handler
      }
      if (this.onmessage) {
        this.onmessage(event);
      }
    };

    this._ws.onerror = (event) => {
      console.error("ReconnectingWebSocket: Error:", event);
      if (this.onerror) {
        this.onerror(event);
      }
    };

    this._ws.onclose = (event) => {
      this._stopPinging(); // Stop heartbeat

      // if it was closed with a normal close code, it means it was closed by the server
      // intentionally, so we don't want to reconnect
      if (event.code === 1000 || event.code === 1001 || this._forcedClose) {
        console.log(
          `ReconnectingWebSocket: Connection closed normally. Code: ${event.code}, Reason: ${event.reason}`,
        );
        if (this.onclose) {
          this.onclose(event);
        }
        return;
      }

      if (this._reconnectAttempts == 0) {
        this._lastDisconnectTime = Date.now();
      }
      this._handleReconnect(event);
    };
  }

  _handleReconnect(event) {
    var more_than_max_time_passed =
      Date.now() - this._lastDisconnectTime < this._maxReconnectTime;
    if (
      this._reconnectAttempts < this._maxReconnectAttempts &&
      more_than_max_time_passed
    ) {
      this._reconnectAttempts++;
      this._totalReconnectAttempts++;
      console.log(
        `ReconnectingWebSocket: Connection lost. Reconnecting with same session... (${this._reconnectAttempts}/${this._maxReconnectAttempts})`,
      );
      if (this.onreconnect) {
        this.onreconnect({
          attempts: this._reconnectAttempts,
          maxAttempts: this._maxReconnectAttempts,
          delay: this._reconnectDelay,
        });
      }
      setTimeout(() => this.connect(), this._reconnectDelay);
    } else {
      console.error(
        `ReconnectingWebSocket: Failed to reconnect after ${this._maxReconnectAttempts} attempts.`,
      );
      if (this.onclose) {
        this.onclose(event);
      }
    }
  }

  /**
   * Starts the ping/pong heartbeat to detect dead connections.
   */
  _startPinging() {
    this._stopPinging(); // Ensure no existing timers are running
    this._pingIntervalId = setInterval(() => {
      if (this.readyState === WebSocket.OPEN) {
        // Use the underlying ws.send to avoid resetting reconnect attempts
        this._ws.send("ping");

        // Set a timeout to wait for the pong. If it doesn't arrive,
        // the connection is considered dead.
        this._pongTimeoutId = setTimeout(() => {
          console.warn(
            "ReconnectingWebSocket: Pong not received in time. Closing connection.",
          );
          if (this._ws) {
            // Close with a custom code. This will trigger the onclose handler,
            // which will then initiate the reconnection logic.
            this._ws.close(4000, "Pong timeout");
          }
        }, this._pongTimeoutDuration);
      }
    }, this._pingIntervalDuration);
  }

  /**
   * Stops the ping/pong heartbeat.
   */
  _stopPinging() {
    if (this._pingIntervalId) {
      clearInterval(this._pingIntervalId);
      this._pingIntervalId = null;
    }
    if (this._pongTimeoutId) {
      clearTimeout(this._pongTimeoutId);
      this._pongTimeoutId = null;
    }
  }

  /**
   * Sends data through the WebSocket connection.
   * @param {string|ArrayBuffer|Blob} data The data to send.
   */
  send(data) {
    if (this.readyState === WebSocket.OPEN) {
      this._ws.send(data);
      this._reconnectAttempts = 0;
    } else {
      throw new Error("WebSocket is not open. readyState: " + this.readyState);
    }
  }

  /**
   * Closes the WebSocket connection permanently.
   * @param {number} [code] The close code.
   * @param {string} [reason] The close reason.
   */
  close(code, reason) {
    this._forcedClose = true;
    this._stopPinging(); // Stop heartbeat on explicit close
    if (this._ws) {
      this._ws.close(code, reason);
    }
  }

  // --- Getters ---

  /** The unique session ID for this connection instance. */
  get sessionId() {
    return this._sessionId;
  }

  /** The current state of the WebSocket connection. */
  get readyState() {
    return this._ws ? this._ws.readyState : WebSocket.CONNECTING;
  }

  /** The URL (including session ID) as resolved by the constructor. */
  get url() {
    // The internal _ws.url is the fully resolved one.
    return this._ws ? this._ws.url : this._url;
  }

  get bufferedAmount() {
    return this._ws ? this._ws.bufferedAmount : 0;
  }

  get extensions() {
    return this._ws ? this._ws.extensions : "";
  }

  get protocol() {
    return this._ws ? this._ws.protocol : "";
  }

  get binaryType() {
    return this._ws ? this._ws.binaryType : "blob";
  }

  set binaryType(type) {
    if (this._ws) {
      this._ws.binaryType = type;
    }
  }
}

// Add WebSocket constants for convenience
ReconnectingWebSocket.CONNECTING = 0;
ReconnectingWebSocket.OPEN = 1;
ReconnectingWebSocket.CLOSING = 2;
ReconnectingWebSocket.CLOSED = 3;

Shiny.createSocket = function () {
  const url = "websocket";
  return new ReconnectingWebSocket(url);
};

/**
 * Gets the WebSocket instance for the current Shiny application.
 *
 * This is a convenience function for accessing the socket which is normally
 * stored at `Shiny.shinyapp.$socket`.
 *
 * @returns {ReconnectingWebSocket | null} The active ReconnectingWebSocket
 *   instance, or null if no connection is established.
 */
function getShinySocket() {
  return Shiny.shinyapp.$socket;
}
