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
   */
  constructor(url, protocols, options = {}) {

    console.log(url);
    // --- Public Interface ---
    this.onopen = null;
    this.onclose = null;
    this.onmessage = null;
    this.onerror = null;
    this.onreconnect = null;

    // --- Internal State ---
    this._protocols = protocols;
    this._ws = null;
    this._reconnectAttempts = 0;
    this._totalReconnectAttempts = 0;
    this._forcedClose = false;

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
      options.reconnectDelay != null ? options.reconnectDelay : 100; // 100ms

    // Initial connection
    this.connect();
  }

  /**
   * Initiates the WebSocket connection.
   */
  connect() {
    console.log(`ReconnectingWebSocket: Connecting to ${this._url}...`);
    this._ws = new WebSocket(`${this._url}&attempt=${this._totalReconnectAttempts}`, this._protocols);

    this._ws.onopen = (event) => {
      console.log(
        `ReconnectingWebSocket: Connection opened with Session ID: ${this._sessionId}`,
      );
      if (this.onopen) {
        this.onopen(event);
      }
    };

    this._ws.onmessage = (event) => {
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

      // if it was closed with a normal close code, it means it was closed by the server
      // intentionally, so we don't want to reconnect
      if (event.code === 1000 || event.code === 1001 || this._forcedClose) {
        console.log(`ReconnectingWebSocket: Connection closed normally. Code: ${event.code}, Reason: ${event.reason}`);
        if (this.onclose) {
          this.onclose(event);
        }
        return;
      }

      this._handleReconnect(event);
    };
  }

  _handleReconnect(event) {
    if (this._reconnectAttempts < this._maxReconnectAttempts) {
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
      console.error(`ReconnectingWebSocket: Failed to reconnect after ${this._maxReconnectAttempts} attempts.`);
      if (this.onclose) {
        this.onclose(event);
      }
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
