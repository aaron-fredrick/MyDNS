# HTTPS Deployment Guide

## Overview

MyDNS does not include built-in TLS termination. Instead, it is designed to run behind a TLS-terminating reverse proxy (nginx, Caddy, Apache, etc.). This approach provides:

- **Simplified certificate management**: Let reverse proxies handle ACME/Let's Encrypt automation
- **Better security**: Battle-tested TLS implementations in mature reverse proxy software
- **Flexibility**: Easy to add additional security layers (WAF, rate limiting, etc.)
- **Performance**: Efficient TLS termination with HTTP/2 support

## Recommended Reverse Proxies

### Caddy (Recommended)

Caddy is the simplest option with automatic HTTPS:

```caddyfile
mydns.example.com {
    reverse_proxy 127.0.0.1:8080
    
    # Security headers
    header {
        X-Content-Type-Options nosniff
        X-Frame-Options DENY
        Referrer-Policy strict-origin-when-cross-origin
        -Server
    }
}
```

### Nginx

```nginx
server {
    listen 443 ssl http2;
    server_name mydns.example.com;

    ssl_certificate /etc/letsencrypt/live/mydns.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/mydns.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Security headers
        add_header X-Content-Type-Options nosniff always;
        add_header X-Frame-Options DENY always;
        add_header Referrer-Policy strict-origin-when-cross-origin always;
    }
}
```

## MyDNS Configuration

Configure MyDNS to listen on localhost only:

```ini
[server]
http_host = 127.0.0.1
http_port = 8080
```

The reverse proxy will handle public HTTPS connections and forward to MyDNS over HTTP on localhost.

## WebSocket Support

The MyDNS dashboard uses WebSockets for real-time updates. Ensure your reverse proxy supports WebSocket upgrades:

**Caddy**: Automatically handles WebSockets

**Nginx**: Add these headers:
```nginx
proxy_http_version 1.1;
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";
```

## Security Considerations

1. **Firewall**: Block direct access to MyDNS HTTP port (8080) from external networks
2. **Certificate management**: Use certbot or Caddy's automatic HTTPS for certificate renewal
3. **HSTS**: Consider adding HSTS headers after initial deployment testing
4. **Rate limiting**: Implement rate limiting at the reverse proxy level for additional protection

## Testing HTTPS Deployment

1. Start MyDNS on localhost:8080
2. Configure reverse proxy to forward to MyDNS
3. Test dashboard access via HTTPS
4. Verify WebSocket connections work
5. Check browser security indicators (lock icon, certificate validity)
