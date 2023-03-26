-- the locals `raw_args` and `path` are provided when included

local function trim(s)
    return s:match('^%s*(.-)%s*$')
end

local function escape(s)
    return "'" .. s:gsub("'", [['"'"']]) .. "'"
end

----------------------
-- Argument parsing --
----------------------

local args = {}
local temp_key = ''
local temp = ''
local brace_count = 0
local escaped = false
for char in raw_args:gmatch('.') do
    if char == '=' and brace_count == 0 then
        temp_key = temp
        temp = ''
        goto continue
    elseif char == ',' and brace_count == 0 then
        args[trim(temp_key)] = trim(temp)
        temp_key = ''
        temp = ''
        goto continue
    elseif char == '{' and not escaped then
        brace_count = brace_count + 1
        if brace_count == 1 then
            goto continue
        end
    elseif char == '}' and not escaped then
        brace_count = brace_count - 1
        if brace_count == 0 then
            goto continue
        end
    elseif char == '\\' then
        escaped = true
        temp = temp .. char
        goto continue
    end
    temp = temp .. char
    escaped = false
    ::continue::
end
if trim(temp_key) ~= '' and trim(temp) ~= '' then
    args[trim(temp_key)] = trim(temp)
end

------------------------------------
-- Constructing output LaTeX code --
------------------------------------

-- build command flags
local command_flags = ''
if args.raw == 'true' then
    command_flags = command_flags .. ' --raw'
end
if args['raw queries'] == 'true' then
    command_flags = command_flags .. ' --raw-queries'
end
if args.ranges ~= nil then
    command_flags = command_flags .. ' --ranges ' .. escape(args.ranges)
end
if args['path prefix'] ~= nil then
    command_flags = command_flags .. ' --filename-strip-prefix ' .. escape(args['path prefix'])
end
if args.fancyvrb ~= nil then
    command_flags = command_flags .. ' --fancyvrb-args ' .. escape(args.fancyvrb)
end

-- begin float if `flaot` is set
if args.float ~= nil then
    tex.print([[\begin{listing}[]] .. args.float .. ']')
end
if args.wrap ~= nil then
    tex.print([[\begin{wrapfloat}{listing}{]] ..
        args.wrap .. '}{' .. (args['wrap width'] or [[0.5\textwidth]]) .. '}')
end

-- call lirstings
local handle, err
if args.ansi == 'true' then
    handle, err = io.popen("lirstings ansi " .. escape(path) .. command_flags)
else
    handle, err = io.popen("lirstings tree-sitter " .. escape(path) .. command_flags)
end
if not handle then
    print(err)
    os.exit(1)
end
for line in (handle:read('*all') .. '\n'):gmatch('(.-)\r?\n') do
    tex.print(line)
end
handle:close()

-- end float and set caption
if args.float ~= nil or args.wrap ~= nil then
    if args.caption ~= nil then
        tex.print([[\caption{]] .. args.caption .. '}')
    end
    if args.label ~= nil then
        tex.print([[\label{]] .. args.label .. '}')
    end
    if args.float ~= nil then
        tex.print([[\end{listing}]])
    else
        tex.print([[\end{wrapfloat}]])
    end
end
