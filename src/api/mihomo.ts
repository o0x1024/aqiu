let baseUrl = "http://127.0.0.1:29090";
let authSecret: string | null = null;

export const setMihomoBaseUrl = (url: string) => {
  baseUrl = url.replace(/\/$/, "");
};

export const setMihomoSecret = (secret?: string | null) => {
  const normalized = secret?.trim();
  authSecret = normalized ? normalized : null;
};

const buildUrl = (path: string) => `${baseUrl}${path}`;
const withAuthHeader = (headers: Record<string, string> = {}) => {
  if (authSecret) {
    return { ...headers, Authorization: `Bearer ${authSecret}` };
  }
  return headers;
};

const withTokenQuery = (path: string) => {
  if (!authSecret) return path;
  const separator = path.includes("?") ? "&" : "?";
  return `${path}${separator}token=${encodeURIComponent(authSecret)}`;
};

const buildWsUrl = (path: string) =>
  buildUrl(withTokenQuery(path)).replace(/^http/, "ws");

const assertOk = async (res: Response) => {
  if (res.ok) return;
  const text = await res.text().catch(() => "");
  const suffix = text ? ` - ${text}` : "";
  throw new Error(`Mihomo API error: ${res.status} ${res.statusText}${suffix}`);
};

export interface Config {
  port: number;
  "socks-port": number;
  "redir-port": number;
  "tproxy-port": number;
  "mixed-port": number;
  "allow-lan": boolean;
  mode: string;
  "log-level": string;
  ipv6: boolean;
  "external-controller"?: string;
  rules?: string[];
  "proxy-groups"?: any[];
  tun?: {
    enable: boolean;
    stack?: string;
    "auto-route"?: boolean;
    "auto-detect-interface"?: boolean;
    "dns-hijack"?: string[];
    device?: string;
  };
}

export interface Proxy {
  name: string;
  type: string;
  history: { time: string; delay: number }[];
  udp: boolean;
  now?: string;
  all?: string[]; // for selector
  url?: string;
  interval?: number;
}

export interface ProxyData {
  proxies: Record<string, Proxy>;
}

export interface Rule {
  type: string;
  payload: string;
  proxy: string;
}

export interface Connection {
  id: string;
  metadata: {
    network: string;
    type: string;
    sourceIP: string;
    destinationIP: string;
    sourcePort: string;
    destinationPort: string;
    host: string;
    processPath?: string;
    process_path?: string;
    process?: string;
  };
  upload: number;
  download: number;
  start: string;
  chains: string[];
  rule: string;
  rulePayload: string;
}

export const api = {
  getConfig: async (): Promise<Config> => {
    const res = await fetch(buildUrl("/configs"), {
      headers: withAuthHeader(),
    });
    await assertOk(res);
    return res.json();
  },

  updateConfig: async (config: Partial<Config>) => {
    const res = await fetch(buildUrl("/configs"), {
      method: "PATCH",
      headers: withAuthHeader({ "Content-Type": "application/json" }),
      body: JSON.stringify(config),
    });
    await assertOk(res);
  },

  getProxies: async (): Promise<ProxyData> => {
    const res = await fetch(buildUrl("/proxies"), {
      headers: withAuthHeader(),
    });
    await assertOk(res);
    return res.json();
  },

  getProxyDelay: async (name: string, url = "http://www.gstatic.com/generate_204", timeout = 5000) => {
    const res = await fetch(buildUrl(`/proxies/${encodeURIComponent(name)}/delay?url=${encodeURIComponent(url)}&timeout=${timeout}`), {
      headers: withAuthHeader(),
    });
    await assertOk(res);
    return res.json();
  },

  selectProxy: async (group: string, name: string) => {
    const res = await fetch(buildUrl(`/proxies/${encodeURIComponent(group)}`), {
      method: "PUT",
      headers: withAuthHeader({ "Content-Type": "application/json" }),
      body: JSON.stringify({ name }),
    });
    await assertOk(res);
  },

  getRules: async (): Promise<{ rules: Rule[] }> => {
    const res = await fetch(buildUrl("/rules"), {
      headers: withAuthHeader(),
    });
    await assertOk(res);
    return res.json();
  },

  getConnections: async (): Promise<{ connections: Connection[] }> => {
    const res = await fetch(buildUrl("/connections"), {
      headers: withAuthHeader(),
    });
    await assertOk(res);
    return res.json();
  },

  closeConnection: async (id: string) => {
    const res = await fetch(buildUrl(`/connections/${id}`), {
      method: "DELETE",
      headers: withAuthHeader(),
    });
    await assertOk(res);
  },

  closeAllConnections: async () => {
    const res = await fetch(buildUrl("/connections"), {
      method: "DELETE",
      headers: withAuthHeader(),
    });
    await assertOk(res);
  },

  // WebSockets
  connectTraffic: (callback: (data: { up: number; down: number }) => void) => {
    const ws = new WebSocket(buildWsUrl("/traffic"));
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      callback(data);
    };
    return ws;
  },

  connectLogs: (level: string, callback: (data: { type: string; payload: string }) => void) => {
    const ws = new WebSocket(buildWsUrl(`/logs?level=${level}`));
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      callback(data);
    };
    return ws;
  },
};
