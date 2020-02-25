# Datadog Badges

A simple badge server that will expose small badges for the status of Datadog monitors.

## Usage

You will need an API Key and an Application Key for each team you want to expose badges for and set them as appropriately named environment variables.

The environment varibale names are derived from the subdomain of `datadoghq.com` that the team uses.
The subdomain is converted to upper case and all non alpha-numeric characters are replaced by underscores.
If your team's Datadog is hosted at `example-team.datadoghq.com` then you would set the environment variables: `EXAMPLE_TEAM_API_KEY` and `EXAMPLE_TEAM_APP_KEY` 

The badges will be available at URLs: `http://hostname:8080/account/{subdomain}/monitors/{monitorId}`.
Using our example again, monitor 12345 would be exposed on `http://hostname:8080/account/example-team/monitors/12345` 

