import { describe, it, expect, beforeEach, afterEach } from "bun:test";
import { api, setMihomoBaseUrl, setMihomoSecret } from "../src/api/mihomo";

type FetchCall = [RequestInfo | URL, RequestInit | undefined];

describe("mihomo api auth", () => {
  let calls: FetchCall[] = [];
  let originalFetch: typeof fetch | undefined;

  beforeEach(() => {
    calls = [];
    originalFetch = globalThis.fetch;
    globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
      calls.push([input, init]);
      return {
        ok: true,
        status: 200,
        statusText: "OK",
        json: async () => ({}),
        text: async () => "",
      } as Response;
    }) as typeof fetch;
    setMihomoBaseUrl("http://127.0.0.1:29090");
    setMihomoSecret(null);
  });

  afterEach(() => {
    if (originalFetch) {
      globalThis.fetch = originalFetch;
    }
  });

  it("does not send Authorization when secret is empty", async () => {
    await api.getConfig();
    const [, init] = calls[0];
    const headers = init?.headers as Record<string, string> | undefined;
    expect(headers?.Authorization).toBeUndefined();
  });

  it("sends Authorization when secret is set", async () => {
    setMihomoSecret("my-secret");
    await api.updateConfig({ mode: "Rule" });
    const [, init] = calls[0];
    const headers = init?.headers as Record<string, string> | undefined;
    expect(headers?.Authorization).toBe("Bearer my-secret");
  });
});

describe("mihomo websocket auth", () => {
  let originalWebSocket: typeof WebSocket | undefined;

  class MockWebSocket {
    static instances: MockWebSocket[] = [];
    url: string;
    onmessage: ((event: { data: string }) => void) | null = null;
    onclose: (() => void) | null = null;
    onerror: (() => void) | null = null;
    constructor(url: string) {
      this.url = url;
      MockWebSocket.instances.push(this);
    }
    close() { }
  }

  beforeEach(() => {
    originalWebSocket = globalThis.WebSocket;
    globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;
    setMihomoBaseUrl("http://127.0.0.1:29090");
    setMihomoSecret("ws-secret");
    MockWebSocket.instances = [];
  });

  afterEach(() => {
    if (originalWebSocket) {
      globalThis.WebSocket = originalWebSocket;
    }
  });

  it("appends token to websocket urls", () => {
    api.connectLogs("debug", () => { });
    expect(MockWebSocket.instances.length).toBe(1);
    expect(MockWebSocket.instances[0].url).toContain("token=ws-secret");
  });
});
