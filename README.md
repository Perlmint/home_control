# Setup traefik

```
traefik.http.routers.hydra.rule=Host(`auth.hydra`)&&!Path(`/login`,`/consent`,`/forward-auth`)
traefik.http.services.hydra.loadbalancer.server.port=4444
traefik.http.middlewares.hydra-auth.forwardauth.address=https://auth.example.com/forward-auth
traefik.http.middlewares.hydra-auth.forwardauth.authRequestHeaders=Authorization

traefik.http.routers.hydra-front.rule=Host(`auth.example.com`)&&Path(`/login`,`/consent`,`/forward-auth`)

traefik.http.routers.hub.rule=Host(`home.example.com`)
traefik.http.routers.hub.middlewares=hydra-auth
```