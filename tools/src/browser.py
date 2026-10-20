"""What a Playwright page reports while it runs, and the engines the harness opens."""

from dataclasses import dataclass, field

from playwright.sync_api import (
    BrowserType,
    ConsoleMessage,
    Error,
    Page,
    Playwright,
    Request,
    Response,
)

ENGINES = ("webkit", "chromium", "firefox")


@dataclass
class PageLog:
    """What a page reported while it ran."""

    console: list[str] = field(default_factory=lambda: [])
    console_errors: list[str] = field(default_factory=lambda: [])
    page_errors: list[str] = field(default_factory=lambda: [])
    failed_requests: list[str] = field(default_factory=lambda: [])

    def problems(self) -> list[str]:
        """Console errors, uncaught exceptions and failed requests, in that order."""
        return self.console_errors + self.page_errors + self.failed_requests

    def watch(self, page: Page) -> None:
        def on_console(message: ConsoleMessage) -> None:
            line = f"{message.type}: {message.text}"
            self.console.append(line)
            if message.type == "error":
                self.console_errors.append(line)

        def on_page_error(error: Error) -> None:
            self.page_errors.append(f"pageerror: {error.message}")

        def on_request_failed(request: Request) -> None:
            self.failed_requests.append(f"requestfailed: {request.url} {request.failure}")

        def on_response(response: Response) -> None:
            if response.status >= 400:
                self.failed_requests.append(f"http {response.status}: {response.url}")

        page.on("console", on_console)
        page.on("pageerror", on_page_error)
        page.on("requestfailed", on_request_failed)
        page.on("response", on_response)


def engine_type(playwright: Playwright, engine: str) -> BrowserType:
    match engine:
        case "webkit":
            return playwright.webkit
        case "chromium":
            return playwright.chromium
        case "firefox":
            return playwright.firefox
        case _:
            raise ValueError(f"unknown engine {engine!r}; one of {', '.join(ENGINES)}")
