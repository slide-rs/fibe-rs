[![Build Status](https://travis-ci.org/slide-rs/fibe-rs.png?branch=master)](https://travis-ci.org/slide-rs/fibe-rs)

### Fiberized task queue in Rust

It is currently very generic and simple. Each task is given an unique handle, and other tasks can use it to specify their dependencies. A task only executes when all dependencies are finished. Any sharing or passing of the data between tasks is user's problem. May the channels help him/her.

The implementation can be further expanded by introducing thread pools or fibers without affecting the interface.

The idea is to see how far this design can go when used in a game. Dog-fooding results will then determine the evolution vector or the termination of this humble experiment.
