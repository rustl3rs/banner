# CAVEAT
This repository represents a very pre-alpha experimental attempt at the CI/CD system that I wish I had. Consider this to be a wish list and a set of goals that would ideally be achieved, instead of something that actually exists.

**Be warned** until such times as this starts producing releases, it is completely unusable.  It may never be usable.

# Banner CI/CD

Banner is a CI/CD system.  I know, yet another CI/CD system.  However Banner is different. Some of it's more unique qualities are:
1. Banner pipelines can be run locally
2. A CI/CD DSL.  That means no YAML. While YAML has served us all well, for Concourse/CircleCI/Drone/etc, it does have it's quirks.
3. Banner has dimensionality. This means you can run the same pipeline across multiple git branches
4. Data Linage. Meaning you can track down what part of the pipeline your commit has reached, and when it got overtaken by another commit.
5. Built in observability; see at a glance what the average time for your pipeline/job/task is. See the failure, error and success rates. Get OTEL data for each pipeline, job and task.
6. Easy sandboxing.
7. Partial completion re-runs.  Task state is kept around for a while; rerun only the steps of your job that failed, and avoid all that time, cpu, and memory wastage.
8. Annotations at all levels.
9. An engine system. Set your context to the environment you run in
    * Kubernetes
    * Local
    * AWS Lambda
    * Azure Functions
    * somewhere else....
10. Event based.
11. Composable.

## Influences

[Concourse](https://concourse-ci.org/) is very much an influence on the thinking here.  We like the way that containers are used to isolate work, but feel that there are things that could fundamentally be improved about all CI/CD tools currently available. We've tried to take inspiration from every current CI/CD tool, the best of every world. Hopefully, we'll hit the spot.

Other influences:
* [Argo CD](https://argo-cd.readthedocs.io/en/stable/)
* [CircleCI](https://circleci.com/)
* [Drone](https://www.drone.io/)
* [GoCD](https://www.gocd.org/)
* [Jenkins](https://www.jenkins.io/)
* [TeamCity](https://www.jetbrains.com/teamcity/)
* [Gitlab Actions](https://gitlab.com)
* [Github Actions](https://github.com)
* [Dagger.io](https://dagger.io); I was directed to this on a [reddit comment](https://www.reddit.com/r/rust/comments/10v0ypk/whats_everyone_working_on_this_week_62023/j7rhyy7/).  While I like what they are trying to do, it doesn't fit much of what I'd like to achieve.

## Requirements

For local running of pipelines:
* Docker
* Banner 🙂

For running in the cloud/data-center:
* Banner server with appropriate engine.


# CI/CD Domain Language

A few things to note;
* trailing comma in lists is optional.

Example:
```file://banner.ban
// With the engine command you can define variables, images and tasks that override the 
// generally used objects in a pipeline.  This extends locally as well as into the cloud
// engines; so engine[k8s] works just as well.
engine[local] {
    config.s3.ACCESS_TOKEN = ${AWS_ACCESS_TOKEN}
}

config.docker.config_file = s3://rustl3rs-build-artefacts/banner/secrets/docker-config.json

// require sets up a resource that is needed by this pipeline/job/task
// and gives it an alias. In this case; `src`
// a require will create a directory named whatever the key is of the key/value pair
require src=git://github.com/rustl3rs/banner;

// require-env will create a variable you can use as an ENV var.
require-env loglevel=debug

// images are additive; so if you add an images in another file, it will get merged with other
// images declarations.  This is a caching directive, and tells the Banner engine to pre-fetch the images.
images = [
    "rustl3rs/rust-build:latest",
]

// Think of Image as a struct/object.  It's mostly a key/value pair thing with a given structure
// see the docs.
/// You can also provide doc comments with a tripple slash (///)
/// These comments will be discoverable in the UI.
let get-authors-image = Image {
    name=rustl3rs/rust-build:latest, 
    mount=[
        src -> "/source-code",
    ],
    env=[
        RUSTLOG=${loglevel},
        RUST_BACKTRACE=1,
    ]
}

// import can be one of:
//     * file://
//     * http://
//     * https://
//     * s3://
//     * git://
import file://check-format.ban
import file://build-debug.ban
import s3://rustl3rs-build-artefacts/banner/common/test.ban

import file://build-artefacts.ban
import file://deploy.ban
import file://integration-tests.ban
import file://performance-tests.ban
import https://cdn.rustl3rs.com/banner/common/jobs/prod-gate.ban

// now, quite frankly, you could put get-authors-image inline, but that would be truly cursed from 
// a formatting perspective. :)
// All `tasks` look a bit like methods that take parameters.  The list of parameters can be found
// in the docs; which don't exist yet... soz!
// Pass on additional event data by outputing json on the last line.
task get-authors(exclude_for: src{branches="feature/*"}, image: get-authors-image, execute: "/bin/bash -c") {
    # and this is a bash script!
    # some super lengthy git command to find the authors of the commit(s)
    echo Done!
    echo 'event_data={"version":"${VERSION}"}'
}

// define the job `unit-test`.
// `[]` define sequential tasks
// `{}` define parallel tasks
// This notation defines what the DAG will look like for a job. Similarly, for whole pipelines.
// NB: annotations are added with a `[tag: ]` and are key/value pairs with an `=` to separate the key from the value
[tag: rustl3rs.com/team=platform-team]
[tag: rustl3rs.com/cost-center=company]
[tag: rustl3rs.com/notification=slack#banner-alerts]
job unit-test() [
    { 
        get-authors,
        check-format,
        build-debug,
    },
    test,
]

// events will give you access to
//   * event
//   * job (syntactic sugar for event.job)
//   * task (syntactic sugar for event.task)
// fields so that appropriate action can be taken.
on_event: task[get-authors].failed {
    if event.Reason == "Failed to fetch image" {
        metrics.FailedImagePull += 1
        if task.tries < 4 {
            sleep 5s
            task.resubmit
        }
    }
}

// define the pipeline.
// `[]` define sequential tasks
// `{}` define parallel tasks
// This notation defines what the DAG will look like for a job. Similarly, for whole pipelines.
// the `with dimensions []` syntax defines over what other constructs dimensionality is applicable to this pipeline
// in this case, we are specifying that the `src`; a git repo; defines cardinality over branches that match our regex.
// Each dimension in the array leads to an explosion in cardinality. Use sparingly.
// deploy(xxx) are jobs with different parameters.  The job names would be 
//   * `deploy-ci`
//   * `deploy-qa`
//   * `deploy-prod`
// respectively.
// if however you wanted to deploy to CI twice you'd get
//   * `deploy-ci-0`
//   * `deploy-ci-1`
pipeline Banner-CI(): dimensions [src.branches=regex("(release/.*|feature/.*)")]
[
    unit-test,
    build-artefacts,
    {
        [ deploy(ci), integration-tests, ],
        [ deploy(qa), performance-tests, ],
    }
    prod-gate,
    deploy(prod),
]

// and this is how the pipeline is triggered.
on_event: commit.repo == src {
    if commit.branch matches regex("(release/.*|feature/.*)") {
        trigger(unit-test);
    }
}
```

## Syntactic sugar

A job and a pipeline are merely syntactic sugar around tasks and events, which are the main constructs of Banner.

Consider the example from above:
```
[tag: rustl3rs.com/team=platform-team]
[tag: rustl3rs.com/cost-center=company]
[tag: rustl3rs.com/notification=slack#banner-alerts]
job unit-test() [
    { 
        get-authors,
        check-format,
        build-debug,
    },
    test,
]
```

This desugars to:
```
on_event: job.start[unit-test] {
    trigger task[get-authors];
    trigger task[check-format];
    trigger task[build-debug];
}

on_event: task.finished[get-authors], task.finished[check-format], task.finished[build-debug] {
    trigger task[test];
}
```