# Datadog Badges

[![Badges! We don't need no stinking badges!](https://img.youtube.com/vi/VqomZQMZQCQ/0.jpg)](https://www.youtube.com/watch?v=VqomZQMZQCQ)

A simple badge server that will expose small badges for the status of Datadog monitors.

Each badge reports: 

* whether the monitor is muted or not
* the current status of the monitor
* how long the current status has been in effect

Example:

![Sample badge](example.svg)

## Usage

You will need an API Key and an Application Key for each team you want to expose badges for and set them as appropriately named environment variables.

The environment varibale names are derived from the subdomain of `datadoghq.com` that the team uses.
The subdomain is converted to upper case and all non alpha-numeric characters are replaced by underscores.
If your team's Datadog is hosted at `example-team.datadoghq.com` then you would set the environment variables: `EXAMPLE_TEAM_DATADOG_API_KEY` and `EXAMPLE_TEAM_DATADOG_APP_KEY` 

The badges will be available at URLs: `http://hostname:8080/account/{subdomain}/monitors/{monitorId}`.
Using our example again, monitor 12345 would be exposed on `http://hostname:8080/account/example-team/monitors/12345` 

The query parameter `q` can be used to filter the results of the status check, using our best guess as to how filters work, e.g. `http://hostname:8080/account/example-team/monitors/12345?q=env%3Aprod` should give the same results as available from datadog at `https://example-team.datadoghq.com/monitors/12345?q=env%3Aprod`. 
Pull Requests welcome if you identify any issues with how we parse this string compared with how Datadog parses it.

The query parameters `w` and `h` can be used to override the SVG document image sizes (the badge will still be auto-sized to content), which can be useful if you are say including the badge in another SVG image.

The query parameter `ts` will always be ignored, so you can safely set this to the current time if you need to force the browser to refresh the image on the page.

The following line options cane be used to modify the server configuration:

```
Usage: datadog-badges [options]

Options:
    -h, --help          print this help menu and exit
    -V, --version       print the version and exit
        --host HOST     the host name to bind to (default: 0.0.0.0)
        --port PORT     the port to bind to (default: 8080)
        --context-root ROOT
                        the context root to serve from (default: /)
        --always-ok     Always return images with status code HTTP/200
```

*NOTE:* The Context Root may not contain `/` so can only be used to configure a single segment.

*NOTE:* By default, the HTTP response for images returned will match the status code returned by the Datadog. 
This can cause confusion for web browsers when you try to access non-existing monitors.
The `--always-ok` option disables this behaviour.

Additionally, the default image response caching can be configured using the `CACHE_TTL_SECONDS` environment variable.
If not specified, or if not a valid unsigned integer, it will default to `15` seconds.

## Docker image

The service is also available as a docker image: [`stephenc/datadog-badges`](https://hub.docker.com/r/stephenc/datadog-badges)

```bash
docker run -e EXAMPLE_TEAM_DATADOG_APP_KEY=... -e EXAMPLE_TEAM_DATADOG_API_KEY=... -p 8080:8080 stephenc/datadog-badges
``` 

