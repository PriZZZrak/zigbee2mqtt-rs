import {afterAll, beforeAll, describe, it, vi} from "vitest";
import fs from "node:fs";
import path from "node:path";
import {createStaticFileServer, sendNotFound} from "../../lib/util/staticFileServer";

// Real srvx static middleware (the pinned app dependency), fake request/response
// objects capture status, headers and body for the HTTP frontend contract.
describe("Static file server dump", () => {
    const dir = process.env.STATIC_DIR as string;
    const dumpPath = process.env.STATIC_DUMP_PATH as string;

    beforeAll(() => {
        fs.mkdirSync(path.join(dir, "assets"), {recursive: true});
        fs.writeFileSync(path.join(dir, "index.html"), "<html>index</html>");
        fs.writeFileSync(path.join(dir, "assets", "app.js"), "console.log('app');");
        fs.writeFileSync(path.join(dir, "assets", "app.js.br"), "compressed-js");
        fs.writeFileSync(path.join(dir, "missing.txt"), "");
    });

    afterAll(() => {
        fs.rmSync(path.join(dir, "missing.txt"), {force: true});
    });

    async function request(
        port: number,
        url: string,
        headers: Record<string, string> = {},
    ): Promise<{status: number; headers: Record<string, string>; body: string}> {
        const http = await import("node:http");
        return await new Promise((resolve, reject) => {
            const request = http.request(
                {host: "127.0.0.1", port, path: url, method: "GET", headers},
                (response) => {
                    const chunks: Buffer[] = [];
                    response.on("data", (chunk) => chunks.push(Buffer.from(chunk)));
                    response.on("end", () => {
                        const responseHeaders: Record<string, string> = {};
                        for (const [key, value] of Object.entries(response.headers)) {
                            if (value !== undefined) {
                                responseHeaders[key.toLowerCase()] = String(value);
                            }
                        }
                        resolve({
                            status: response.statusCode ?? 0,
                            headers: responseHeaders,
                            body: Buffer.concat(chunks).toString("base64"),
                        });
                    });
                },
            );
            request.on("error", reject);
            request.end();
        });
    }

    it("dumps responses", async () => {
        const errors: string[] = [];
        const staticServer = createStaticFileServer(dir, (message) => errors.push(message));
        const {createServer} = await import("node:http");
        const server = createServer((req, res) => staticServer(req, res));
        await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
        const address = server.address();
        const port = typeof address === "object" && address ? address.port : 0;
        const cases: Array<{id: string; url: string; headers?: Record<string, string>}> = [
            {id: "index", url: "/"},
            {id: "asset", url: "/assets/app.js"},
            {id: "asset-br", url: "/assets/app.js", headers: {"accept-encoding": "br"}},
            {id: "missing", url: "/nope.txt"},
        ];
        const responses: Record<string, unknown> = {};
        for (const testCase of cases) {
            responses[testCase.id] = await request(port, testCase.url, testCase.headers);
        }
        await new Promise<void>((resolve) => server.close(() => resolve()));

        const notFound: Record<string, string> = {};
        await new Promise<void>((resolve) => {
            const response: any = {
                headersSent: false,
                setHeader: (key: string, value: any) => {
                    notFound[key.toLowerCase()] = String(value);
                },
                writeHead: () => {},
                end: () => resolve(),
            };
            sendNotFound({method: "GET", url: "/nope.txt"} as any, response);
        });

        fs.writeFileSync(dumpPath, JSON.stringify({responses, not_found: notFound, errors}, null, 2));
    }, 30000);
});
