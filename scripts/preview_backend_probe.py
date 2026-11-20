#!/usr/bin/env python3
"""Probe original allocator/start flow with inert graphics/session/FD adapters.

This does not launch a compositor, open a GPU or establish a rendered lab profile.
Original source and compiled reference programs exist only in TemporaryDirectory.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import urllib.request

sys.dont_write_bytecode = True
from scalar_registration_probe import block

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "data/preview-backend-sources.json"
OUTPUT = ROOT / "data/preview-backend-fixtures.json"
HEADER = r'''
#include <algorithm>
#include <cstdint>
#include <format>
#include <functional>
#include <iostream>
#include <memory>
#include <string>
#include <vector>
template<class T>using SP=std::shared_ptr<T>;
template<class T,class...A>SP<T>makeShared(A&&...a){return std::make_shared<T>(std::forward<A>(a)...);}
namespace Hyprutils::Memory{template<class T>using CSharedPointer=std::shared_ptr<T>;}
namespace Aquamarine {
'''
ADAPTER = r'''
struct SPollFD {int fd;std::function<void()> callback;SPollFD(int fd,std::function<void()> f):fd(fd),callback(f){}};
struct IBackendImplementation {
 int notifications=0;
 virtual ~IBackendImplementation()=default;
 virtual eBackendType type()=0;virtual bool start()=0;
 virtual std::vector<SP<SPollFD>>pollFDs()=0;virtual int drmFD()=0;
 virtual void onReady(){++notifications;}
};
struct CHeadlessBackend: IBackendImplementation {
 struct {struct{int get(){return 42;}}timerfd;}timers;
 eBackendType type()override{return AQ_BACKEND_HEADLESS;}
 bool start()override;std::vector<SP<SPollFD>>pollFDs()override;
 int drmFD()override;int drmRenderNodeFD();void dispatchTimers(){}
};
struct FixtureBackend:IBackendImplementation {
 eBackendType kind;bool starts;int fd;bool polls;
 FixtureBackend(eBackendType kind,bool starts,int fd,bool polls):kind(kind),starts(starts),fd(fd),polls(polls){}
 eBackendType type()override{return kind;}bool start()override{return starts;}
 std::vector<SP<SPollFD>>pollFDs()override{return polls?std::vector<SP<SPollFD>>{makeShared<SPollFD>(43,[](){})}:std::vector<SP<SPollFD>>{};}
 int drmFD()override{return fd;}
};
struct CSession {int notifications=0;void onReady(){++notifications;}std::vector<SP<SPollFD>>pollFDs(){return {makeShared<SPollFD>(44,[](){})};}};
struct IAllocator{};
inline bool allocatorSucceeds=true;inline int allocations=0;
struct CGBMAllocator {
 static SP<IAllocator> create(int,int){++allocations;return allocatorSucceeds?makeShared<IAllocator>():nullptr;}
};
inline std::string backendTypeToName(eBackendType type){return std::to_string(int(type));}
struct CBackend {
 std::vector<SBackendImplementationOptions>implementationOptions;
 std::vector<SP<IBackendImplementation>>implementations;
 std::vector<SP<SPollFD>>sessionFDs;
 SP<IAllocator>primaryAllocator;SP<CSession>session;bool ready=false;int self=0,reopens=0;bool reopenSucceeds=true;
 void log(eBackendLogLevel,const std::string&){}
 int reopenDRMNode(int){++reopens;return reopenSucceeds?45:-1;}
 bool start();
};
}
'''
MAIN = r'''
using namespace Aquamarine;
void run(const char*name,std::vector<SP<IBackendImplementation>> implementations,bool reopen=true,bool allocation=true,bool mandatory=false,bool session=false){
 CBackend backend;backend.implementations=implementations;backend.reopenSucceeds=reopen;allocatorSucceeds=allocation;allocations=0;
 for(auto&impl:implementations){SBackendImplementationOptions option;option.backendType=impl->type();option.backendRequestMode=mandatory?AQ_BACKEND_REQUEST_MANDATORY:AQ_BACKEND_REQUEST_IF_AVAILABLE;backend.implementationOptions.push_back(option);}
 if(session)backend.session=makeShared<CSession>();
 const bool accepted=backend.start();int notifications=0;for(auto&impl:implementations)notifications+=impl->notifications;
 std::cout<<"{\"name\":\""<<name<<"\",\"accepted\":"<<(accepted?"true":"false")<<",\"backend_ready\":"<<(backend.ready?"true":"false")
 <<",\"allocator\":"<<(backend.primaryAllocator?"true":"false")<<",\"reopens\":"<<backend.reopens<<",\"allocations\":"<<allocations
 <<",\"remaining\":"<<backend.implementations.size()<<",\"notifications\":"<<notifications<<",\"session_notifications\":"<<(backend.session?backend.session->notifications:0)<<"}\n";
}
auto h(){return makeShared<CHeadlessBackend>();}
auto f(eBackendType type,bool starts,int fd,bool polls){return makeShared<FixtureBackend>(type,starts,fd,polls);}
int main(){
 run("headless-only",{h()});
 run("headless-failed-drm-filtered",{h(),f(AQ_BACKEND_DRM,false,-1,false)});
 run("headless-no-device",{h(),f(AQ_BACKEND_DRM,true,-1,true)});
 run("headless-inert-device",{h(),f(AQ_BACKEND_DRM,true,7,true)});
 run("reopen-refused",{h(),f(AQ_BACKEND_DRM,true,7,true)},false);
 run("allocator-refused",{h(),f(AQ_BACKEND_DRM,true,7,true)},true,false);
 run("mandatory-failure",{h(),f(AQ_BACKEND_DRM,false,-1,false)},true,true,true);
 run("optional-failure-nonempty-poll-retained",{h(),f(AQ_BACKEND_DRM,false,7,true)});
 run("null-only",{f(AQ_BACKEND_NULL,true,-1,false)});
 run("null-before-headless",{f(AQ_BACKEND_NULL,true,-1,false),h()});
 run("headless-before-null",{h(),f(AQ_BACKEND_NULL,true,-1,false)});
 run("empty",{});
 run("none-started",{f(AQ_BACKEND_DRM,false,7,true)});
 run("all-poll-fds-empty",{f(AQ_BACKEND_DRM,true,7,false)});
 run("first-allocator-failure-no-second-try",{f(AQ_BACKEND_DRM,true,7,true),f(AQ_BACKEND_WAYLAND,true,8,true)},true,false);
 run("inert-device-session",{h(),f(AQ_BACKEND_DRM,true,7,true)},true,true,false,true);
 CHeadlessBackend headless;std::cout<<"{\"headless_fds\":["<<headless.drmFD()<<","<<headless.drmRenderNodeFD()<<"]}\n";
}
'''


def fetch(row):
    data = urllib.request.urlopen(row["url"], timeout=20).read(2_000_001)
    if len(data) != row["bytes"] or hashlib.sha256(data).hexdigest() != row["sha256"]:
        raise RuntimeError(f"Pinned preview source changed: {row['url']}")
    return (row["package"], row["version"], row["path"]), data.decode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text())
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        sources = dict(pool.map(fetch, manifest["sources"]))
    observations, policies = {}, {}
    with tempfile.TemporaryDirectory(prefix="hye-preview-reference-") as directory:
        directory = Path(directory)
        for version in ["0.15.0", "0.15.1"]:
            source = sources["aquamarine", version, "src/backend/Backend.cpp"]
            header = sources["aquamarine", version, "include/aquamarine/backend/Backend.hpp"]
            headless = sources["aquamarine", version, "src/backend/Headless.cpp"]
            enums = "".join(block(header, f"enum {name}", ";") for name in ["eBackendType", "eBackendRequestMode", "eBackendLogLevel"])
            options = block(header, "struct SBackendImplementationOptions", ";")
            cpp = HEADER + enums + options + ADAPTER + "using namespace Aquamarine;\n"
            cpp += block(source, "Aquamarine::SBackendImplementationOptions::SBackendImplementationOptions()")
            cpp += block(source, "bool Aquamarine::CBackend::start()")
            for signature in ["bool Aquamarine::CHeadlessBackend::start()", "std::vector<SP<SPollFD>> Aquamarine::CHeadlessBackend::pollFDs()", "int Aquamarine::CHeadlessBackend::drmFD()", "int Aquamarine::CHeadlessBackend::drmRenderNodeFD()"]:
                cpp += block(headless, signature)
            # Original Hyprland backend request construction; no factory, GPU,
            # environment, sessions or other compositor setup is executed here.
            for index, hyprland in enumerate(["0.56.2", "audited-main"]):
                text = sources["hyprland", hyprland, "src/Compositor.cpp"]
                start = text.index("std::vector<Aquamarine::SBackendImplementationOptions> implementations;")
                end = text.index("m_aqBackend = CBackend::create(implementations, options);", start)
                cpp += f"auto policy{index}(){{" + text[start:end] + "return implementations;}\n"
            # Print policy separately from the start-branch observations.
            tail = MAIN.replace(" CHeadlessBackend headless;", r''' for(int i=0;i<2;++i){auto options=i?policy1():policy0();std::cout<<"{\"policy\":"<<i<<",\"options\":[";bool comma=false;for(auto&option:options){if(comma)std::cout<<",";comma=true;std::cout<<"["<<int(option.backendType)<<","<<int(option.backendRequestMode)<<"]";}std::cout<<"]}\n";}
 CHeadlessBackend headless;''')
            path, binary = directory / f"probe-{version}.cpp", directory / f"probe-{version}"
            path.write_text(cpp + tail)
            subprocess.run(["c++", "-std=c++23", "-O0", str(path), "-o", str(binary)], check=True, timeout=60)
            output = subprocess.run([str(binary)], capture_output=True, text=True, check=True, timeout=5)
            rows = [json.loads(line) for line in output.stdout.splitlines()]
            observations[version] = [row for row in rows if "name" in row]
            observations[version].append(next(row for row in rows if "headless_fds" in row))
            current = [row["options"] for row in rows if "policy" in row]
            if policies and list(policies.values()) != current:
                raise RuntimeError("Backend enum identities differ between reference versions")
            policies = dict(zip(["0.56.2", "audited-main"], current))
    report = {"format": "hye-preview-backend-observations-1", "source_manifest_sha256": hashlib.sha256(MANIFEST.read_bytes()).hexdigest(),
              "scope": "Original CBackend start and headless start/FD methods; original Hyprland request construction. Inert session/timer/poll/DRM-reopen/GBM-allocation adapters; no GPU, compositor, frame or input runs.",
              "lab_profile_certified": False, "hyprland_requested_backends": policies, "observations": observations}
    text = json.dumps(report, indent=2) + "\n"
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text() != text:
            raise SystemExit("Preview reference observations differ; inspect pinned behavior")
    else:
        OUTPUT.write_text(text)
    print("Verified 32 original backend-start observations, headless descriptors and both compositor request policies; no rendered profile certified")


if __name__ == "__main__":
    main()
