
# Seeed #

Seeed (with 3 'e's) : a simple yet powerful scripting language to install a remote server.

There are many other tools out there (ansible, chef, puppet, ...) 

## Installation ##

``cargo install seeed``

## language features ##

### A seeed script sample

```
# this a comment

## next is a remote block (command that would be executed on a the remote server

+
| apt-get update
| apt-get install apache2
| a2en_mod rewrite
+ 

## next is a variable assignation, variable type is a string

let username = "johnsnow"

## variable assignation, with a here document

let apache_config = <<<APACHECONF
<server>
// config file shorten for the sake of simplicity
</serv>
APACHECONF>>>

## call a built-in function (upload)

upload($apache_config, "/etc/apache2/site-available/default")

| systemctl restart apache

# that was a single-line remote block

```

## running a _seeed_ script

``seed --target debian@my.server.com -sudo ./01_setup.seeed``

## TODOs ##

Things that I'm planning to add to _seeed_ in a (near) future revision:

- better error handling
- array type and corresponding for loop
- use more string slices instead of Strings
- flags on remote block that tunes the behavior of the remote block (like muting the output, capturing the output to a variable, etc ...)
- templating support on heredocs and remote blocks
- load variables content from the local filesystem using yaml files
- variables object type
- more builtin functions:
  - _random()_ to generate a random string / password
  - _download()_ to download a file