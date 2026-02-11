user      := "atareao"
name      := "expulsabotrs"
version   := `vampus show`
go_format := '{{.Repository}}:{{.Tag}}'

# list all commands
list:
    @just --list

# build and tag
build:
    echo {{version}}
    echo {{name}}
    docker build -t {{user}}/{{name}}:{{version}} \
                 -t {{user}}/{{name}}:latest \
                 .

# tag latest
tag:
    docker tag {{user}}/{{name}}:{{version}} {{user}}/{{name}}:latest

# push all tags
push:
    docker push --all-tags {{user}}/{{name}}

# remove images
rmi:
    docker images --format "{{go_format}}" --filter "reference=atareao/expulsabotrs" | tail -n +4 | xargs -r docker rmi

# list images
lsi:
    docker images --format "{{go_format}}" --filter "reference=atareao/expulsabotrs"

buildx:
    #!/usr/bin/env bash
    #--platform linux/arm/v7,linux/arm64/v8,linux/amd64 \
    docker buildx build \
           --push \
           --platform linux/arm/v7,linux/arm64/v8,linux/amd64 \
           --tag {{user}}/{{name}}:{{version}} .

run:
    docker run --rm \
               --init \
               --name croni \
               --init \
               --env_file croni.env \
               -v ${PWD}/crontab:/crontab \
               {{user}}/{{name}}:{{version}}

sh:
    docker run --rm \
               -it \
               --name croni \
               --init \
               --env-file croni.env \
               -v ${PWD}/crontab:/crontab \
               {{user}}/{{name}}:{{version}} \
               sh

