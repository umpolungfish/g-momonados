"""Read and update complete research records in the shared Markdown notebook."""
import re
from pathlib import Path
import yaml

NOTEBOOK = Path(__file__).with_name('godel_relationships.md')
Loader = getattr(yaml, 'CSafeLoader', yaml.SafeLoader)


class Dumper(getattr(yaml, 'CSafeDumper', yaml.SafeDumper)):
    pass


def represent_list(dumper, values):
    compact = all(not isinstance(v, (dict, list)) for v in values)
    compact = compact or all(isinstance(v, list) and all(not isinstance(x, (dict, list)) for x in v) for v in values)
    return dumper.represent_sequence('tag:yaml.org,2002:seq', values, flow_style=compact)


Dumper.add_representer(list, represent_list)
Dumper.add_representer(tuple, represent_list)


def record_name(path):
    name = Path(path).name
    if name.endswith('.json'):
        name = name[:-5]
    if not re.fullmatch(r'[A-Za-z0-9_-]+', name):
        raise ValueError('record name must contain letters, digits, underscores, or hyphens')
    return name


def render_record(name, value):
    body = yaml.dump(value, Dumper=Dumper, allow_unicode=True, sort_keys=False, width=120)
    longest = max((len(m.group()) for m in re.finditer(r'`+', body)), default=0)
    fence = '`' * max(4, longest + 1)
    return (f'<!-- godel-record: {name} -->\n<a id="record-{name}"></a>\n\n'
            f'## {name}\n\n{fence}yaml\n{body}{fence}\n\n'
            f'<!-- /godel-record: {name} -->\n')


def decode_record(section):
    opening = re.search(r'(?m)^(`{4,})yaml\n', section)
    if opening is None:
        raise ValueError('record has no structured Markdown block')
    end = section.index('\n' + opening.group(1) + '\n', opening.end())
    return yaml.load(section[opening.end():end], Loader=Loader)


def record_span(text, name):
    start = text.index(f'<!-- godel-record: {name} -->')
    closing = f'<!-- /godel-record: {name} -->'
    end = text.index(closing, start) + len(closing)
    if text[end:end+1] == '\n':
        end += 1
    return start, end


def read_record(path):
    name = record_name(path)
    text = NOTEBOOK.read_text()
    start, end = record_span(text, name)
    return decode_record(text[start:end])


def write_record(path, value):
    name = record_name(path)
    text = NOTEBOOK.read_text()
    section = render_record(name, value)
    try:
        start, end = record_span(text, name)
    except ValueError:
        text += '\n' + section
    else:
        previous = decode_record(text[start:end])
        if previous == decode_record(section):
            return
        revision = 1
        while f'<!-- godel-record: {name}-revision-{revision} -->' in text:
            revision += 1
        archive = render_record(f'{name}-revision-{revision}', previous)
        text = text[:start] + section + text[end:] + '\n' + archive
    NOTEBOOK.write_text(text)
