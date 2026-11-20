// Editor-owned text conversions for the pinned complex-value model.
// No Lua execution or original Hyprland implementation is embedded here.
#include <algorithm>
#include <charconv>
#include <cmath>
#include <cfenv>
#include <clocale>
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <limits>
#include <locale>
#include <sstream>
#include <string>
#include <string_view>
#include <system_error>
#include <type_traits>

static_assert(sizeof(float)==4 && sizeof(double)==8);
static_assert(std::numeric_limits<float>::is_iec559 && std::numeric_limits<double>::is_iec559);
extern "C" size_t hye_complex_text_profile(unsigned char* output,size_t capacity) {
    if(!output||!capacity)return 0;
#if defined(__GLIBCXX__)
    try {
        const int n=std::snprintf(reinterpret_cast<char*>(output),capacity,"libstdc++:%u;ieee32/64;complex-text-1",static_cast<unsigned>(__GLIBCXX__));
        return n>0 && static_cast<size_t>(n)<capacity ? static_cast<size_t>(n):0;
    } catch(...) {return 0;}
#else
    return 0;
#endif
}
extern "C" int hye_complex_context_supported() {
#if defined(__GLIBCXX__)
    try {
        const char* ctype=std::setlocale(LC_CTYPE,nullptr);
        return ctype&&(std::strcmp(ctype,"C")==0||std::strcmp(ctype,"POSIX")==0)&&std::locale().name()=="C"&&std::fegetround()==FE_TONEAREST;
    }catch(...){return 0;}
#else
    return 0;
#endif
}
#if defined(__GLIBCXX__)
template<class T>static bool number(std::string_view text,T& out) {
    if(text.empty())return false;
    int base=10;
    if constexpr(std::is_integral_v<T>) {
        if(text.size()>=2 && text[0]=='0' && (text[1]=='x'||text[1]=='X')){base=16;text.remove_prefix(2);}
        const auto result=std::from_chars(text.data(),text.data()+text.size(),out,base);
        return result.ec==std::errc() && result.ptr==text.data()+text.size();
    } else {
        const auto result=std::from_chars(text.data(),text.data()+text.size(),out);
        return result.ec==std::errc() && result.ptr==text.data()+text.size();
    }
}
static bool white(char c){return c==' '||c=='\t'||c=='\n'||c=='\r'||c=='\f'||c=='\v';}
static std::string_view trim(std::string_view text) {
    while(!text.empty()&&white(text.front()))text.remove_prefix(1);
    while(!text.empty()&&white(text.back()))text.remove_suffix(1);
    return text;
}
static bool hex(std::string_view text,uint64_t& value) {
    if(text.substr(0,2)=="0x")return number(text,value);
    std::string prefixed="0x";prefixed.append(text);
    return number(prefixed,value);
}
// 0 accepted; 1 native bad value; 2 undefined/unprofiled conversion; 3 limit/error.
static int color(std::string_view text,uint32_t& out) {
    uint64_t packed=0;
    if(!text.empty()&&text.front()=='#') {
        text.remove_prefix(1);
        if(text.size()!=3&&text.size()!=6&&text.size()!=8)return 1;
        uint64_t values[4]={0,0,0,255};
        const size_t width=text.size()==3?1:2;
        for(size_t i=0;i<text.size()/width;++i)if(!hex(text.substr(i*width,width),values[i]))return 1;
        if(width==1)for(size_t i=0;i<3;++i)values[i]|=values[i]<<4;
        packed=(values[3]<<24)|(values[0]<<16)|(values[1]<<8)|values[2];
    } else if(text.substr(0,2)=="0x") {
        if(!hex(text,packed))return 1;
    } else if((text.substr(0,5)=="rgba("||text.substr(0,4)=="rgb(")&&text.back()==')') {
        const bool alpha=text.substr(0,5)=="rgba(";
        const size_t prefix=alpha?5:4;
        auto body=trim(text.substr(prefix,text.size()-prefix-1));
        const auto commas=std::count(body.begin(),body.end(),',');
        if(commas==(alpha?3:2)) {
            uint8_t channels[3]={};
            for(size_t i=0;i<3;++i){const size_t at=body.find(',');if(!number(trim(body.substr(0,at)),channels[i]))return 1;if(at!=std::string_view::npos)body.remove_prefix(at+1);}
            uint64_t a=255;
            if(alpha) {
                float opacity=0;
                if(!number(trim(body),opacity))return 1;
                const float scaled=std::floor(opacity*255.0F);
                // Original float-to-unsigned cast has no defined result here.
                if(!std::isfinite(scaled)||scaled<0||static_cast<long double>(scaled)>=18446744073709551616.0L)return 2;
                a=static_cast<uint64_t>(scaled);
            }
            packed=(a<<24)|(uint64_t(channels[0])<<16)|(uint64_t(channels[1])<<8)|channels[2];
        } else if(body.size()==(alpha?8:6)) {
            if(!hex(body,packed))return 1;
            if(alpha)packed=(packed>>8)+(0x1000000ULL*(packed&255));
            else packed+=0xff000000;
        } else return 1;
    } else {
        if(text.empty()||text.front()=='-'||!std::all_of(text.begin(),text.end(),[](char c){return c>='0'&&c<='9';}))return 1;
        uint32_t value=0;if(!number(text,value))return 1;packed=value;
    }
    // Every source path constructs CHyprColor from the low ARGB bytes. Its
    // normalized binary64 channels multiply back exactly for all 256 values.
    out=static_cast<uint32_t>(packed);
    return 0;
}
extern "C" int hye_complex_text(int kind,const unsigned char* input,size_t length,uint32_t* output) {
    if(!hye_complex_context_supported())return 2;
    if(!input||!output||length>65536)return 3;
    for(size_t i=0;i<length;++i)if(input[i]>=128)return 2; // upstream ctype signed-char preconditions
    try {
        const std::string_view text(reinterpret_cast<const char*>(input),length);
        if(kind==1)return color(text,*output);
        if(kind==2) {
            std::string data(text);std::replace(data.begin(),data.end(),',',' ');
            std::istringstream stream(data);double x=0,y=0;std::string trailing;
            if(!(stream>>x>>y)||(stream>>trailing))return 1;
            const float a=static_cast<float>(x),b=static_cast<float>(y);
            std::memcpy(output,&a,sizeof(a));std::memcpy(output+1,&b,sizeof(b));return 0;
        }
        return 3;
    }catch(...){return 3;}
}
#else
extern "C" int hye_complex_text(int,const unsigned char*,size_t,uint32_t*) { return 2; }
#endif
