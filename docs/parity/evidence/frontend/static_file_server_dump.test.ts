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
        fs.writeFileSync(path.join(dir, "assets", "app.js.gz"), "gzipped-js");
        fs.writeFileSync(path.join(dir, "about.html"), "<html>about</html>");
        fs.writeFileSync(path.join(dir, "missing.txt"), "");
        // Deterministic validators: ETag/Last-Modified must match the Rust
        // candidate, so both sides pin the same mtime.
        for (const name of ["index.html", "assets/app.js", "assets/app.js.br", "assets/app.js.gz", "about.html"]) {
            fs.utimesSync(path.join(dir, name), 1700000000, 1700000000);
        }
    });

    afterAll(() => {
        fs.rmSync(path.join(dir, "missing.txt"), {force: true});
    });

    async function request(
        port: number,
        url: string,
        headers: Record<string, string> = {},
        method = "GET",
    ): Promise<{status: number; headers: Record<string, string>; body: string}> {
        const http = await import("node:http");
        return await new Promise((resolve, reject) => {
            const request = http.request(
                {host: "127.0.0.1", port, path: url, method, headers},
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
        const cases: Array<{id: string; url: string; headers?: Record<string, string>; method?: string}> = [
            {id: "index", url: "/"},
            {id: "asset", url: "/assets/app.js"},
            {id: "asset-br", url: "/assets/app.js", headers: {"accept-encoding": "br"}},
            {id: "asset-gzip", url: "/assets/app.js", headers: {"accept-encoding": "gzip"}},
            {id: "asset-preference", url: "/assets/app.js", headers: {"accept-encoding": "gzip, br"}},
            {id: "head-index", url: "/", method: "HEAD"},
            {id: "range", url: "/assets/app.js", headers: {range: "bytes=0-4"}},
            {id: "range-unsatisfiable", url: "/assets/app.js", headers: {range: "bytes=100-"}},
            {id: "extensionless", url: "/about"},
            {id: "missing", url: "/nope.txt"},
        ];
        const responses: Record<string, unknown> = {};
        for (const testCase of cases) {
            responses[testCase.id] = await request(
                port,
                testCase.url,
                testCase.headers,
                testCase.method,
            );
        }
        const etag = (responses.asset as any)?.headers?.etag;
        if (etag) {
            responses.conditional = await request(port, "/assets/app.js", {"if-none-match": etag});
        }
        const lastModified = (responses.asset as any)?.headers?.["last-modified"];
        if (lastModified) {
            responses["conditional-since"] = await request(port, "/assets/app.js", {
                "if-modified-since": lastModified,
            });
            responses["range-if-range"] = await request(port, "/assets/app.js", {
                range: "bytes=0-4",
                "if-range": lastModified,
            });
        }
        responses["range-stale-if-range"] = await request(port, "/assets/app.js", {
            range: "bytes=0-4",
            "if-range": '"stale"',
        });
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
