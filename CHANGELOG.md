# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

* #572 Rephrase misleading documentation (`invalid`)

## 0.14.1

### Fixed

* #569 Properly handle records without an PPN (`filter`)

## 0.14.0

### Fixed

* #563 Fix false positives of `not in` operator

### Added

* #557 Add short variant for reduce option (`filter`)
* #534 Add `cat` snapshot tests
* #524 Add `invalid` snapshot tests
* #525 Add long help (`invalid`)

### Changed

* #562 Use `Reader` and `ReaderBuilder` instead of `BufReadExt`
* #532 Improve performance of `cat` command
* #522 Use `BufReadExt` to process records (`invalid`)
* #523 Move reader/writer logic to config (`invalid`)
* #537 Stabilize `cat` command
* #538 Stabilize `completions` command
* #554 Stabilize `count` command
* #566 Cleanup release workflow
