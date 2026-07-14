/* global process */

const host = "127.0.0.1";
const port = Number.parseInt(process.env.PORT || "4175", 10);

import("node:http").then(({ createServer }) => {
  const server = createServer((_request, response) => {
    response.writeHead(200, { "content-type": "text/plain; charset=utf-8" });
    response.end("Desktop Manager conflicting lockfile fixture");
  });
  server.listen(port, host, () => {
    process.stdout.write(`conflicting-lockfiles fixture listening on http://${host}:${port}\n`);
  });
});
