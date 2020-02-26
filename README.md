# Datadog Badges

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
If your team's Datadog is hosted at `example-team.datadoghq.com` then you would set the environment variables: `EXAMPLE_TEAM_API_KEY` and `EXAMPLE_TEAM_APP_KEY` 

The badges will be available at URLs: `http://hostname:8080/account/{subdomain}/monitors/{monitorId}`.
Using our example again, monitor 12345 would be exposed on `http://hostname:8080/account/example-team/monitors/12345` 

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
docker run -e EXAMPLE_TEAM_APP_KEY=... -e EXAMPLE_TEAM_API_KEY=... -p 8080:8080 stephenc/datadog-badges
``` 

