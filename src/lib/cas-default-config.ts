import type { WebhookConfig } from './webhook.js';

/** Example webhook URL for development presets. */
export const URL_WEBHOOK_DEV = 'https://example.com/webhook';

/** Example webhook URL for production presets. */
export const URL_WEBHOOK_ONLINE = 'https://example.com/webhook';

/** Placeholder custom header for the webhook modal. */
export const HEADER_CUSTOM_DEFAULT = 'X-API-Key: your-api-key-here';

/** Default CAS webhook config used by presets, defaultConfig, and store merge. */
export function defaultCasWebhookConfig(): WebhookConfig {
  return {
    enabled: false,
    url: URL_WEBHOOK_DEV,
    method: 'POST',
    throttle_ms: 1000,
    include_raw: false,
    custom_headers: HEADER_CUSTOM_DEFAULT,
    body_format: 'json',
    form_template: 'device-id={{port}}&reading={{weight}}&timestamp={{timestamp}}',
    scales_code: '',
    company_id: '',
    mode_webhook: 'DEV',
    url_online: URL_WEBHOOK_ONLINE,
    send_filter: { enabled: false, field: '', value: '' },
  };
}
