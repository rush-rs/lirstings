# Script to update lirstings default config file
# using the `onedark.nvim` light theme colors

import re
import requests
import json

colors_map = {
    'Fg': 'fg',
    'LightGrey': 'light_grey',
    'Grey': 'grey',
    'Red': 'red',
    'Cyan': 'cyan',
    'Yellow': 'yellow',
    'Orange': 'orange',
    'Green': 'green',
    'Blue': 'blue',
    'Purple': 'purple',
}
code_style = {
    'comments': 'italic',
    'keywords': None,
    'functions': None,
    'strings': None,
    'variables': None,
}
allowed_fmts = ['italic', 'bold', 'underline', 'strikethrough']

groups_re = re.compile(
    r'\["@([\w.]+)"\]\s*=\s*(?:colors\.(\w+)|\{fg\s*=\s*c\.(\w+),\s*fmt\s*=\s*(?:cfg.code_style.(\w+)|(["'
    + r"'])(.*)\5)\})"
)
colors_re = re.compile(r'(\w+) = "(#[a-fA-F0-9]{6})",')

print('Fetching `highlights.lua`...')
highlights_lua = requests.get(
    'https://raw.githubusercontent.com/navarasu/onedark.nvim/master/lua/onedark/highlights.lua'
).text


print('Fetching `palette.lua`...')
palette_lua = requests.get(
    'https://raw.githubusercontent.com/navarasu/onedark.nvim/master/lua/onedark/palette.lua'
).text

print('Fetching complete')

colors = {
    match.group(1): match.group(2)
    for match in colors_re.finditer(
        palette_lua.split('light = {')[1].split('}')[0]
    )
}

theme = {}

for name, value in colors.items():
    theme[name] = value

for match in groups_re.finditer(highlights_lua):
    if match.group(2) is not None:
        theme[match.group(1)] = '$' + colors_map[match.group(2)]
    elif (
        match.group(4) is not None and code_style[match.group(4)] is None
    ) or (match.group(6) is not None and match.group(6) not in allowed_fmts):
        theme[match.group(1)] = '$' + match.group(3)
    else:
        theme[match.group(1)] = {'link': match.group(3)}
        if (
            match.group(4) is not None
            and code_style[match.group(4)] is not None
        ):
            theme[match.group(1)][code_style[match.group(4)]] = True
        elif match.group(6) in allowed_fmts:
            theme[match.group(1)][match.group(6)] = True

with open('src/default_config.json', 'r') as config_file:
    curr_conf = json.load(config_file)

for key, value in theme.items():
    curr_conf['theme'][key] = value

with open('src/default_config.json', 'w') as config_file:
    json.dump(
        curr_conf,
        config_file,
        indent=2,
    )
