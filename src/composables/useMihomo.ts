import { ref, reactive, watch } from 'vue';
import { api, setMihomoBaseUrl, setMihomoSecret, type Config, type Proxy, type Rule, type Connection } from '../api/mihomo';
import type { CoreStatus } from '../api/tauri';
import { useCore } from './useCore';

// Global state
const config = ref<Config | null>(null);
const proxies = ref<Record<string, Proxy>>({});
const rules = ref<Rule[]>([]);
const connections = ref<Connection[]>([]);
const logs = ref<{ time: string; type: string; payload: string }[]>([]);
const MAX_LOGS = 2000;
const coreRunning = ref<boolean>(false);
const manualSecret = ref<string | null>(localStorage.getItem('aqiu-mihomo-secret'));

const traffic = reactive({ up: 0, down: 0 });
const totalTraffic = reactive({ up: 0, down: 0 });
let lastTrafficSampleAt = 0;

let trafficWs: WebSocket | null = null;
let logWs: WebSocket | null = null;
let connInterval: ReturnType<typeof setInterval> | null = null;
let trafficReconnectTimer: ReturnType<typeof setTimeout> | null = null;
let logsReconnectTimer: ReturnType<typeof setTimeout> | null = null;
let trafficReconnectDelay = 2000;
let logsReconnectDelay = 2000;
let coreWatchInitialized = false;
let isReconnecting = false;

// Forward declaration
let openSockets: () => void;

const resetReconnectDelays = () => {
    trafficReconnectDelay = 2000;
    logsReconnectDelay = 2000;
};

const nextDelay = (current: number) => Math.min(current * 2, 30000);

const clearReconnectTimers = () => {
    if (trafficReconnectTimer) {
        clearTimeout(trafficReconnectTimer);
        trafficReconnectTimer = null;
    }
    if (logsReconnectTimer) {
        clearTimeout(logsReconnectTimer);
        logsReconnectTimer = null;
    }
};

const scheduleReconnect = (type: "traffic" | "logs") => {
    if (!coreRunning.value) {
        console.log(`WebSocket ${type}: Core not running, skip reconnect`);
        return;
    }
    if (type === "traffic" && trafficReconnectTimer) return;
    if (type === "logs" && logsReconnectTimer) return;
    
    const delay = type === "traffic" ? trafficReconnectDelay : logsReconnectDelay;
    console.log(`WebSocket ${type}: Scheduling reconnect in ${delay}ms`);
    
    const timer = setTimeout(() => {
        if (type === "traffic") {
            trafficReconnectTimer = null;
            trafficReconnectDelay = nextDelay(trafficReconnectDelay);
        } else {
            logsReconnectTimer = null;
            logsReconnectDelay = nextDelay(logsReconnectDelay);
        }
        if (coreRunning.value) {
            openSockets();
        }
    }, delay);
    
    if (type === "traffic") {
        trafficReconnectTimer = timer;
    } else {
        logsReconnectTimer = timer;
    }
};

export function useMihomo() {
    const fetchConfig = async () => {
        if (!coreRunning.value) return;
        try {
            config.value = await api.getConfig();
        } catch (e) {
            console.error("Failed to fetch config", e);
        }
    };

    const fetchProxies = async (retries = 3, delay = 1000): Promise<boolean> => {
        if (!coreRunning.value) return false;

        for (let i = 0; i < retries; i++) {
            try {
                const data = await api.getProxies();
                // Check if we actually got any proxies (Mihomo might return empty object during init)
                if (data.proxies && Object.keys(data.proxies).length > 0) {
                    proxies.value = data.proxies;
                    return true;
                }
                console.log(`Fetch proxies attempt ${i + 1}: No proxies found yet, retrying...`);
            } catch (e) {
                console.error(`Failed to fetch proxies (attempt ${i + 1}):`, e);
            }
            if (i < retries - 1) {
                await new Promise(resolve => setTimeout(resolve, delay));
            }
        }
        return false;
    };

    const fetchRules = async () => {
        if (!coreRunning.value) return;
        try {
            const data = await api.getRules();
            rules.value = data.rules;
        } catch (e) {
            console.error("Failed to fetch rules", e);
        }
    };

    const fetchConnections = async () => {
        if (!coreRunning.value) return;
        try {
            const data = await api.getConnections();
            connections.value = Array.isArray(data.connections) ? data.connections : [];
        } catch (e) {
            console.error("Failed to fetch connections", e);
        }
    };

    const selectProxy = async (groupName: string, proxyName: string) => {
        try {
            await api.selectProxy(groupName, proxyName);
            await fetchProxies(); // Refresh state
        } catch (e) {
            console.error("Failed to select proxy", e);
        }
    };

    const testLatency = async (proxyName: string) => {
        try {
            const res = await api.getProxyDelay(proxyName);
            // Ideally update local state with new delay
            if (proxies.value[proxyName]) {
                proxies.value[proxyName].history.push({ time: new Date().toISOString(), delay: res.delay });
            }
        } catch (e) {
            console.error("Failed to test latency", e);
        }
    }


    const closeSockets = () => {
        console.log('WebSocket: Closing all connections');
        isReconnecting = false;
        clearReconnectTimers();
        
        if (trafficWs) {
            trafficWs.onclose = null; // Prevent reconnect on intentional close
            trafficWs.onerror = null;
            trafficWs.close();
            trafficWs = null;
        }
        if (logWs) {
            logWs.onclose = null;
            logWs.onerror = null;
            logWs.close();
            logWs = null;
        }
        
        resetReconnectDelays();
    };

    const startConnectionsPolling = () => {
        if (connInterval) return;
        connInterval = setInterval(fetchConnections, 2000);
    };

    const stopConnectionsPolling = () => {
        if (connInterval) {
            clearInterval(connInterval);
            connInterval = null;
        }
    };

    openSockets = () => {
        if (!coreRunning.value) {
            console.log('WebSocket: Core not running, skip opening sockets');
            return;
        }
        
        // Avoid duplicate connections
        if (isReconnecting) {
            console.log('WebSocket: Already reconnecting, skip');
            return;
        }
        
        try {
            // Only open traffic WS if not already connected
            if (!trafficWs || trafficWs.readyState === WebSocket.CLOSED) {
                console.log('WebSocket: Opening traffic connection');
                trafficWs = api.connectTraffic((data) => {
                    const now = Date.now();
                    if (lastTrafficSampleAt > 0) {
                        const deltaSeconds = (now - lastTrafficSampleAt) / 1000;
                        if (deltaSeconds > 0) {
                            totalTraffic.up += data.up * deltaSeconds;
                            totalTraffic.down += data.down * deltaSeconds;
                        }
                    }
                    lastTrafficSampleAt = now;
                    traffic.up = data.up;
                    traffic.down = data.down;
                });
                
                trafficWs.onopen = () => {
                    console.log('WebSocket: Traffic connection opened');
                    resetReconnectDelays();
                };
                trafficWs.onclose = (event) => {
                    console.log('WebSocket: Traffic connection closed', event.code);
                    trafficWs = null;
                    if (coreRunning.value && event.code !== 1000) {
                        scheduleReconnect("traffic");
                    }
                };
                trafficWs.onerror = (error) => {
                    console.error('WebSocket: Traffic connection error', error);
                    trafficWs?.close();
                };
            }

            // Only open logs WS if not already connected
            if (!logWs || logWs.readyState === WebSocket.CLOSED) {
                console.log('WebSocket: Opening logs connection');
                logWs = api.connectLogs('debug', (data) => {
                    logs.value.unshift({
                        time: new Date().toLocaleTimeString(),
                        type: data.type,
                        payload: data.payload
                    });
                    if (logs.value.length > MAX_LOGS) logs.value.pop();
                });
                
                logWs.onopen = () => {
                    console.log('WebSocket: Logs connection opened');
                    resetReconnectDelays();
                };
                logWs.onclose = (event) => {
                    console.log('WebSocket: Logs connection closed', event.code);
                    logWs = null;
                    if (coreRunning.value && event.code !== 1000) {
                        scheduleReconnect("logs");
                    }
                };
                logWs.onerror = (error) => {
                    console.error('WebSocket: Logs connection error', error);
                    logWs?.close();
                };
            }
        } catch (e) {
            console.error('Failed to establish WebSocket connections', e);
        }
    };

    const applyCoreStatus = (status: CoreStatus) => {
        coreRunning.value = status.running;
        if (status.api_endpoint) {
            setMihomoBaseUrl(status.api_endpoint);
        }
        setMihomoSecret(manualSecret.value || status.api_secret);
        if (!status.running) {
            closeSockets();
            stopConnectionsPolling();
            proxies.value = {};
            config.value = null;
            totalTraffic.up = 0;
            totalTraffic.down = 0;
            lastTrafficSampleAt = 0;
            return;
        }
        fetchConfig();
        fetchProxies();
        fetchRules();
        startConnectionsPolling();
        openSockets();
    };

    if (!coreWatchInitialized) {
        coreWatchInitialized = true;
        const { coreStatus } = useCore();
        watch(coreStatus, (status) => {
            if (status) {
                applyCoreStatus(status);
                return;
            }
            applyCoreStatus({
                running: false,
                version: null,
                config_path: null,
                api_host: '127.0.0.1',
                api_port: 29090,
                api_endpoint: 'http://127.0.0.1:29090',
                api_secret: null,
                uptime_seconds: null,
                message: null,
            });
        }, { immediate: true });
    }

    const closeAllConnections = async () => {
        try {
            await api.closeAllConnections();
            await fetchConnections();
        } catch (e) {
            console.error("Failed to close connections", e);
        }
    };

    const updateConfig = async (newConfig: Partial<import('../api/mihomo').Config>) => {
        try {
            await api.updateConfig(newConfig);
            await fetchConfig();
        } catch (e) {
            console.error("Failed to update config", e);
        }
    };

    return {
        config,
        proxies,
        rules,
        connections,
        logs,
        traffic,
        totalTraffic,
        coreRunning,
        manualSecret,
        setManualSecret: (secret: string | null) => {
            const normalized = secret?.trim();
            manualSecret.value = normalized ? normalized : null;
            if (manualSecret.value) {
                localStorage.setItem('aqiu-mihomo-secret', manualSecret.value);
            } else {
                localStorage.removeItem('aqiu-mihomo-secret');
            }
            setMihomoSecret(manualSecret.value);
        },
        selectProxy,
        testLatency,
        fetchProxies,
        closeAllConnections,
        updateConfig
    };
}
