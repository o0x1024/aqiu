import { type, platform } from '@tauri-apps/plugin-os';

// Cache platform info (synchronous in plugin-os v2)
let _isMac: boolean | null = null;
let _isWindows: boolean | null = null;
let _isLinux: boolean | null = null;
let _platformInitialized = false;

/**
 * Initialize platform detection.
 * Call this once at app startup.
 */
export function initPlatform(): void {
	if (_platformInitialized) return;
	try {
		const osType = type();
		_isMac = osType === 'macos';
		_isWindows = osType === 'windows';
		_isLinux = osType === 'linux';
		_platformInitialized = true;
	} catch (e) {
		console.error('Failed to detect platform:', e);
		// Fallback to platform() which should also be sync
		try {
			const p = platform();
			_isMac = p === 'macos';
			_isWindows = p === 'windows';
			_isLinux = p === 'linux';
			_platformInitialized = true;
		} catch {
			// Last resort fallback
			_isMac = false;
			_isWindows = false;
			_isLinux = false;
			_platformInitialized = true;
		}
	}
}

/**
 * Check if running on macOS.
 * Must call initPlatform() first.
 */
export function isMac(): boolean {
	if (!_platformInitialized) {
		initPlatform();
	}
	return _isMac ?? false;
}

/**
 * Check if running on Windows.
 * Must call initPlatform() first.
 */
export function isWindows(): boolean {
	if (!_platformInitialized) {
		initPlatform();
	}
	return _isWindows ?? false;
}

/**
 * Check if running on Linux.
 * Must call initPlatform() first.
 */
export function isLinux(): boolean {
	if (!_platformInitialized) {
		initPlatform();
	}
	return _isLinux ?? false;
}

