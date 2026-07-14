/* global process */

import { createServer } from "node:http";

const host = "127.0.0.1";
const port = Number.parseInt(process.env.PORT ?? "4173", 10);

const server = createServer((_request, response) => {
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end("<h1>Desktop Manager synthetic Vite fixture</h1>");
});

server.listen(port, host, () => {
  process.stdout.write("fixture-server listening on http://" + host + ":" + port + "\n");
});

const stop = () => server.close(() => process.exit(0));
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
