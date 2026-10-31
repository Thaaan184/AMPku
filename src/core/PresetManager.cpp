#include "core/PresetManager.hpp"
#include "nlohmann/json.hpp"
#include <fstream>
#include <iostream>

using json = nlohmann::json;

namespace guitarfx {

PresetManager::PresetManager(std::string presets_dir)
    : presets_dir_(std::move(presets_dir)) {
    ensure_factory_presets();
}

void PresetManager::ensure_factory_presets() {
    std::filesystem::create_directories(presets_dir_);

    auto write_if_missing = [this](const std::string& filename, const json& j) {
        std::filesystem::path p = std::filesystem::path(presets_dir_) / filename;
        if (!std::filesystem::exists(p)) {
            std::ofstream f(p);
            if (f.is_open()) {
                f << j.dump(2);
            }
        }
    };

    // 1. Clean
    json clean_j = {
        {"name", "Clean Sparkle"},
        {"description", "Pristine transparent clean tone with warm algorithmic space."},
        {"effects", {
            {"input_gain", {{"enabled", true}, {"gain_db", 0.0}}},
            {"noise_gate", {{"enabled", false}, {"threshold_db", -55.0}}},
            {"overdrive", {{"enabled", false}, {"drive", 0.15}, {"tone", 0.6}, {"level", 0.8}}},
            {"distortion", {{"enabled", false}, {"drive", 0.3}, {"tone", 0.5}, {"level", 0.7}}},
            {"amp_sim", {{"enabled", true}, {"voicing", 0}, {"gain", 0.3}, {"bass", 0.5}, {"mid", 0.55}, {"treble", 0.7}}},
            {"eq", {{"enabled", true}, {"low_db", 1.0}, {"mid_db", 0.0}, {"high_db", 2.0}}},
            {"delay", {{"enabled", false}, {"time_ms", 280.0}, {"feedback", 0.25}, {"mix", 0.2}}},
            {"reverb", {{"enabled", true}, {"decay", 0.45}, {"damping", 0.3}, {"mix", 0.22}}},
            {"limiter", {{"enabled", true}, {"ceiling_db", -0.3}}}
        }}
    };
    write_if_missing("clean.json", clean_j);

    // 2. Blues
    json blues_j = {
        {"name", "Texas Blues"},
        {"description", "Dynamic vintage overdrive with crunch tube amp voicing and slapback delay."},
        {"effects", {
            {"input_gain", {{"enabled", true}, {"gain_db", 2.0}}},
            {"noise_gate", {{"enabled", true}, {"threshold_db", -50.0}}},
            {"overdrive", {{"enabled", true}, {"drive", 0.35}, {"tone", 0.55}, {"level", 0.85}}},
            {"distortion", {{"enabled", false}, {"drive", 0.4}, {"tone", 0.5}, {"level", 0.7}}},
            {"amp_sim", {{"enabled", true}, {"voicing", 1}, {"gain", 0.55}, {"bass", 0.5}, {"mid", 0.65}, {"treble", 0.6}}},
            {"eq", {{"enabled", true}, {"low_db", 2.0}, {"mid_db", 3.0}, {"high_db", 1.5}}},
            {"delay", {{"enabled", true}, {"time_ms", 130.0}, {"feedback", 0.20}, {"mix", 0.18}}},
            {"reverb", {{"enabled", true}, {"decay", 0.4}, {"damping", 0.4}, {"mix", 0.20}}},
            {"limiter", {{"enabled", true}, {"ceiling_db", -0.3}}}
        }}
    };
    write_if_missing("blues.json", blues_j);

    // 3. Crunch
    json crunch_j = {
        {"name", "Modern Crunch"},
        {"description", "Punchy rhythm crunch with mid-forward bite."},
        {"effects", {
            {"input_gain", {{"enabled", true}, {"gain_db", 1.0}}},
            {"noise_gate", {{"enabled", true}, {"threshold_db", -45.0}}},
            {"overdrive", {{"enabled", true}, {"drive", 0.50}, {"tone", 0.55}, {"level", 0.8}}},
            {"distortion", {{"enabled", false}, {"drive", 0.4}, {"tone", 0.5}, {"level", 0.7}}},
            {"amp_sim", {{"enabled", true}, {"voicing", 1}, {"gain", 0.70}, {"bass", 0.55}, {"mid", 0.70}, {"treble", 0.55}}},
            {"eq", {{"enabled", true}, {"low_db", 1.0}, {"mid_db", 2.5}, {"high_db", 0.5}}},
            {"delay", {{"enabled", false}, {"time_ms", 300.0}, {"feedback", 0.3}, {"mix", 0.2}}},
            {"reverb", {{"enabled", true}, {"decay", 0.35}, {"damping", 0.5}, {"mix", 0.15}}},
            {"limiter", {{"enabled", true}, {"ceiling_db", -0.3}}}
        }}
    };
    write_if_missing("crunch.json", crunch_j);

    // 4. Rock
    json rock_j = {
        {"name", "High-Gain Lead"},
        {"description", "Aggressive diode distortion, high-gain amp voicing, and tight noise gate."},
        {"effects", {
            {"input_gain", {{"enabled", true}, {"gain_db", 3.0}}},
            {"noise_gate", {{"enabled", true}, {"threshold_db", -42.0}}},
            {"overdrive", {{"enabled", false}, {"drive", 0.3}, {"tone", 0.5}, {"level", 0.8}}},
            {"distortion", {{"enabled", true}, {"drive", 0.65}, {"tone", 0.60}, {"level", 0.75}}},
            {"amp_sim", {{"enabled", true}, {"voicing", 2}, {"gain", 0.85}, {"bass", 0.60}, {"mid", 0.50}, {"treble", 0.65}}},
            {"eq", {{"enabled", true}, {"low_db", 3.0}, {"mid_db", -1.0}, {"high_db", 2.0}}},
            {"delay", {{"enabled", true}, {"time_ms", 380.0}, {"feedback", 0.35}, {"mix", 0.25}}},
            {"reverb", {{"enabled", true}, {"decay", 0.50}, {"damping", 0.4}, {"mix", 0.25}}},
            {"limiter", {{"enabled", true}, {"ceiling_db", -0.3}}}
        }}
    };
    write_if_missing("rock.json", rock_j);

    // 5. Ambient
    json ambient_j = {
        {"name", "Lush Ambient"},
        {"description", "Dreamy spatial textures with long stereo ping-pong delay and cathedral reverb."},
        {"effects", {
            {"input_gain", {{"enabled", true}, {"gain_db", 0.0}}},
            {"noise_gate", {{"enabled", false}, {"threshold_db", -60.0}}},
            {"overdrive", {{"enabled", false}, {"drive", 0.2}, {"tone", 0.5}, {"level", 0.8}}},
            {"distortion", {{"enabled", false}, {"drive", 0.3}, {"tone", 0.5}, {"level", 0.7}}},
            {"amp_sim", {{"enabled", true}, {"voicing", 0}, {"gain", 0.25}, {"bass", 0.45}, {"mid", 0.60}, {"treble", 0.65}}},
            {"eq", {{"enabled", true}, {"low_db", 0.0}, {"mid_db", 1.0}, {"high_db", 3.0}}},
            {"delay", {{"enabled", true}, {"time_ms", 480.0}, {"feedback", 0.50}, {"mix", 0.40}}},
            {"reverb", {{"enabled", true}, {"decay", 0.85}, {"damping", 0.25}, {"mix", 0.45}}},
            {"limiter", {{"enabled", true}, {"ceiling_db", -0.3}}}
        }}
    };
    write_if_missing("ambient.json", ambient_j);
}

std::vector<std::string> PresetManager::list_presets() const {
    std::vector<std::string> names;
    if (!std::filesystem::exists(presets_dir_)) return names;

    for (const auto& entry : std::filesystem::directory_iterator(presets_dir_)) {
        if (entry.is_regular_file() && entry.path().extension() == ".json") {
            names.push_back(entry.path().stem().string());
        }
    }
    return names;
}

bool PresetManager::load_preset(const std::string& name_or_path, DSPChain& chain) {
    std::filesystem::path p = name_or_path;
    if (!p.has_extension()) {
        p = std::filesystem::path(presets_dir_) / (name_or_path + ".json");
    }

    std::ifstream f(p);
    if (!f.is_open()) return false;

    try {
        json j;
        f >> j;

        if (j.contains("effects")) {
            const auto& eff = j["effects"];

            if (eff.contains("input_gain")) {
                const auto& ig = eff["input_gain"];
                chain.input_gain().set_enabled(ig.value("enabled", true));
                chain.input_gain().set_gain_db(ig.value("gain_db", 0.0f));
            }
            if (eff.contains("noise_gate")) {
                const auto& ng = eff["noise_gate"];
                chain.noise_gate().set_enabled(ng.value("enabled", false));
                chain.noise_gate().set_threshold_db(ng.value("threshold_db", -50.0f));
            }
            if (eff.contains("overdrive")) {
                const auto& od = eff["overdrive"];
                chain.overdrive().set_enabled(od.value("enabled", false));
                chain.overdrive().set_drive(od.value("drive", 0.35f));
                chain.overdrive().set_tone(od.value("tone", 0.5f));
                chain.overdrive().set_level(od.value("level", 0.8f));
            }
            if (eff.contains("distortion")) {
                const auto& ds = eff["distortion"];
                chain.distortion().set_enabled(ds.value("enabled", false));
                chain.distortion().set_drive(ds.value("drive", 0.5f));
                chain.distortion().set_tone(ds.value("tone", 0.5f));
                chain.distortion().set_level(ds.value("level", 0.7f));
            }
            if (eff.contains("amp_sim")) {
                const auto& as = eff["amp_sim"];
                chain.amp_sim().set_enabled(as.value("enabled", true));
                chain.amp_sim().set_voicing(static_cast<AmpVoicing>(as.value("voicing", 1)));
                chain.amp_sim().set_gain(as.value("gain", 0.5f));
                chain.amp_sim().set_bass(as.value("bass", 0.5f));
                chain.amp_sim().set_mid(as.value("mid", 0.6f));
                chain.amp_sim().set_treble(as.value("treble", 0.55f));
            }
            if (eff.contains("eq")) {
                const auto& eq = eff["eq"];
                chain.eq().set_enabled(eq.value("enabled", true));
                chain.eq().set_low_db(eq.value("low_db", 0.0f));
                chain.eq().set_mid_db(eq.value("mid_db", 2.0f));
                chain.eq().set_high_db(eq.value("high_db", 1.0f));
            }
            if (eff.contains("delay")) {
                const auto& dl = eff["delay"];
                chain.delay().set_enabled(dl.value("enabled", false));
                chain.delay().set_time_ms(dl.value("time_ms", 320.0f));
                chain.delay().set_feedback(dl.value("feedback", 0.35f));
                chain.delay().set_mix(dl.value("mix", 0.25f));
            }
            if (eff.contains("reverb")) {
                const auto& rv = eff["reverb"];
                chain.reverb().set_enabled(rv.value("enabled", true));
                chain.reverb().set_decay(rv.value("decay", 0.5f));
                chain.reverb().set_damping(rv.value("damping", 0.4f));
                chain.reverb().set_mix(rv.value("mix", 0.25f));
            }
            if (eff.contains("limiter")) {
                const auto& lm = eff["limiter"];
                chain.limiter().set_enabled(lm.value("enabled", true));
                chain.limiter().set_ceiling_db(lm.value("ceiling_db", -0.3f));
            }
        }

        current_preset_name_ = p.stem().string();
        return true;
    } catch (...) {
        return false;
    }
}

bool PresetManager::save_preset(const std::string& name, const DSPChain& chain, const std::string& description) {
    std::filesystem::path p = std::filesystem::path(presets_dir_) / (name + ".json");

    json j = {
        {"name", name},
        {"description", description},
        {"effects", {
            {"input_gain", {{"enabled", chain.get_effects()[0]->is_enabled()}, {"gain_db", const_cast<DSPChain&>(chain).input_gain().get_gain_db()}}},
            {"noise_gate", {{"enabled", chain.get_effects()[1]->is_enabled()}, {"threshold_db", const_cast<DSPChain&>(chain).noise_gate().get_threshold_db()}}},
            {"overdrive", {{"enabled", chain.get_effects()[2]->is_enabled()}, {"drive", const_cast<DSPChain&>(chain).overdrive().get_drive()}, {"tone", const_cast<DSPChain&>(chain).overdrive().get_tone()}, {"level", const_cast<DSPChain&>(chain).overdrive().get_level()}}},
            {"distortion", {{"enabled", chain.get_effects()[3]->is_enabled()}, {"drive", const_cast<DSPChain&>(chain).distortion().get_drive()}, {"tone", const_cast<DSPChain&>(chain).distortion().get_tone()}, {"level", const_cast<DSPChain&>(chain).distortion().get_level()}}},
            {"amp_sim", {{"enabled", chain.get_effects()[4]->is_enabled()}, {"voicing", static_cast<int>(const_cast<DSPChain&>(chain).amp_sim().get_voicing())}, {"gain", const_cast<DSPChain&>(chain).amp_sim().get_gain()}, {"bass", const_cast<DSPChain&>(chain).amp_sim().get_bass()}, {"mid", const_cast<DSPChain&>(chain).amp_sim().get_mid()}, {"treble", const_cast<DSPChain&>(chain).amp_sim().get_treble()}}},
            {"eq", {{"enabled", chain.get_effects()[5]->is_enabled()}, {"low_db", const_cast<DSPChain&>(chain).eq().get_low_db()}, {"mid_db", const_cast<DSPChain&>(chain).eq().get_mid_db()}, {"high_db", const_cast<DSPChain&>(chain).eq().get_high_db()}}},
            {"delay", {{"enabled", chain.get_effects()[6]->is_enabled()}, {"time_ms", const_cast<DSPChain&>(chain).delay().get_time_ms()}, {"feedback", const_cast<DSPChain&>(chain).delay().get_feedback()}, {"mix", const_cast<DSPChain&>(chain).delay().get_mix()}}},
            {"reverb", {{"enabled", chain.get_effects()[7]->is_enabled()}, {"decay", const_cast<DSPChain&>(chain).reverb().get_decay()}, {"damping", const_cast<DSPChain&>(chain).reverb().get_damping()}, {"mix", const_cast<DSPChain&>(chain).reverb().get_mix()}}},
            {"limiter", {{"enabled", chain.get_effects()[8]->is_enabled()}, {"ceiling_db", const_cast<DSPChain&>(chain).limiter().get_ceiling_db()}}}
        }}
    };

    std::ofstream f(p);
    if (!f.is_open()) return false;
    f << j.dump(2);
    current_preset_name_ = name;
    return true;
}

} // namespace guitarfx
