"""Snapshot exact build sources, including the shared Jupiter.UI working files, without changing them."""
import hashlib,json,subprocess,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
output=ROOT/'artifacts/LichessLocal-0.1.0-source.zip'
entries=[]
with zipfile.ZipFile(output,'w',zipfile.ZIP_DEFLATED) as archive:
    for folder,prefix,include_new in [(ROOT,'lichess-local',True),(ROOT.parent/'jupiter-ui','jupiter-ui',False)]:
        args=['git','-C',str(folder),'ls-files','-z']
        if include_new:args+=['--cached','--others','--exclude-standard']
        names=subprocess.check_output(args).decode().split('\0')
        for name in sorted(set(names)):
            if not name:continue
            file=(folder/name).resolve()
            if not file.is_relative_to(folder) or not file.is_file():continue
            data=file.read_bytes();destination=prefix+'/'+name.replace('\\','/')
            archive.writestr(destination,data);entries.append({'path':destination,'sha256':hashlib.sha256(data).hexdigest()})
    archive.writestr('source-manifest.json',json.dumps(entries,indent=2))
print(f'Packaged {len(entries)} source files: {output}')
