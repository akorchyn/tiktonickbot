#!/usr/bin/env python3
"""
Generates enum declarations for the commands using toml configs stored in commands_config directory.
"""

import toml
import os
import datetime

current_dirname = os.path.dirname(__file__)
config_directory = os.path.join(current_dirname, "commands_config")
result_file = os.path.join(
    current_dirname, "../src/processing/bot/commands.rs")

print("Starting loading data from configs...")
print(f"Config directory: {config_directory}")
print(f"Result file: {result_file}")

configs = []
for filename in os.listdir(config_directory):
    if filename.endswith(".toml"):
        with open(os.path.join(config_directory, filename)) as f:
            config = toml.load(f)
            print(f"Found config for commands: {config['api']['name']}")
            configs.append(config)

timestamp = datetime.datetime.now()
print("Starting generating enum...")
with open(result_file, "w") as f:
    header = f"""
// This file is generated by generate_commands.py in. Generation date is {timestamp}
// The proper way to change commands is to edit commands_config directory and run the script.

use teloxide::utils::command::BotCommand;


"""
    f.write(header)

    for config in configs:
        api_name = config["api"]["name"]
        commands = config["commands"]["list"]
        print(f"Processing api {api_name}...")
        print(f"Found {commands} commands...")

        enum_name = f"{api_name}Commands"
        description = config["commands"]["description"]
        f.write("#[derive(BotCommand, Clone)]\n")
        f.write(
            f'#[command(rename = "lowercase", description = "{description}", parse_with = "split")]\n')
        f.write(f"pub(super) enum {enum_name} {{\n")
        for command in commands:
            command_settings = config["commands"][command]
            tg_command = command_settings["tg_command"]
            description = command_settings["description"]
            args = command_settings["arguments"]
            command_name = ''.join(
                x.capitalize() or '_' for x in command.split('_'))

            if len(args):
                command_name = f"{command_name}({', '.join(args)})"
            f.write(
                f'#[command(rename="{tg_command}", description="{description}")]\n{command_name},\n')
        f.write("}\n\n")

print("Formatting generated file...")
os.system(f"rustfmt {result_file}")