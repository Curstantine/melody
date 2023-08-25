# Specification

Contains the specification both the frontend and backend should follow. This documentation serves the purpose of streamlining the workflow to avoid unnecessary implementations and side-effects.

- [Specification](#specification)
  - [Directories](#directories)
    - [Config Directory](#config-directory)
  - [Persistence](#persistence)
    - [Primary (SQLite)](#primary-sqlite)
    - [Settings (TOML)](#settings-toml)
  - [Components](#components)
    - [Library](#library)
  - [Recovering](#recovering)

## Directories

### Config Directory

The config directory contains all the configuration and metadata files.

Paths:

- Linux: `${XDG_CONFIG_HOME}/melody/`

## Persistence

Melody uses a combination of databases and files to store its data.\
The data must stored in a way that it can be easily backed up and restored.

All these persistence methods must be implemented only in the backend, which then can be accessed by the frontend using tauri's IPC.

### Primary (SQLite)

Melody uses sqlite as its primary database.\
This database contains all the runtime data, such as libraries, playlists,
queues, play counts, tracks, albums and artists.

### Settings (TOML)

All the settings must be stored in a TOML file under [`config_directory/settings.toml`](#config-directory).\
These settings must be editable by the user using both the application and by opening the file in a text editor.

## Components

### Library

A library can define tracks, albums, artists, playlists, queues and play counts.\
There can be multiple libraries, but only one must be active at a given time.

Stored in: Libraries are stored in the primary sqlite database.

## Recovering

I'll have to THINK about this.
