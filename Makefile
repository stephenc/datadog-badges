##@ Default goal

.PHONY: build
build: ## Builds the docker image
	docker build --tag stephenc/datadog-badges "$(dir $(realpath $(firstword $(MAKEFILE_LIST))))"

.PHONY: publish
publish: build ## Publishes the docker image to DockerHub
	docker push stephenc/datadog-badges

.PHONY: exec
exec: build ## Runs the docker image
	docker run -ti --rm stephenc/docker-git-java8-maven-vim bash

# Self documenting makefile
# Sections start with "hash hash at space" comments
# Documented goals end with "hash hash space" comments
.PHONY: help
help: ## Show this help.
	@printf "Usage:\n  make <target>\n"
	@    @IFS=$$'\n' ; \
	     help_lines=(`fgrep -h "##" $(MAKEFILE_LIST) | sed -e '/fgrep/d;/^##[^@] /d;s/^##@/##/;s/\\$$//;s/:.*##/ ##/' `); \
	     for help_line in $${help_lines[@]}; do \
	         IFS=$$'#' ; \
	         help_split=($$help_line) ; \
	         help_command=`echo $${help_split[0]} | sed -e 's/^ *//' -e 's/ *$$//'` ; \
	         help_info=`echo $${help_split[2]} | sed -e 's/^ *//' -e 's/ *$$//'` ; \
	         if [[ -z $${help_command} ]] ; then \
	             if [[ ! -z $${help_info} ]] ; then \
	                 printf "\n$$help_info\n\n" ; \
	             fi ; \
	         else \
	             printf "  %-30s %s\n" $$help_command $$help_info ; \
	         fi \
	     done ;

